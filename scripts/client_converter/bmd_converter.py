#!/usr/bin/env python3
"""
bmd_converter.py
================

Pure Python converter for MU Online BMD 3D model files to GLTF Binary (GLB) format.

The BMD format has been reverse-engineered from:
  - cpp/MuClient5.2/source/ZzzBMD.cpp (Open/Open2 parsing)
  - cpp/MuClient5.2/source/ZzzBMD.h (struct definitions)
  - cpp/MuClientTools16/_src_/Core/BMD_SMD.cpp (FixUpBones, Bmd2Smd)
  - cpp/MuClientTools16/_src_/Core/BMD.h (struct definitions)

Usage:
    python3 bmd_converter.py \\
        --bmd-root cpp/MuClient5.2/bin/Data \\
        --output-root rust/assets/data \\
        --format glb \\
        --force --verbose \\
        --report rust/assets/reports/models_report.json
"""

from __future__ import annotations

import argparse
import json
import logging
import math
import os
import struct
import subprocess
import sys
import tempfile
import time
from dataclasses import dataclass, field
from pathlib import Path
from typing import Dict, List, Optional, Tuple

class BmdParseError(Exception):
    pass

# ---------------------------------------------------------------------------
# Constants
# ---------------------------------------------------------------------------

Q_PI = 3.14159265358979323846

# BMD struct sizes (MSVC-aligned, on-disk)
SIZEOF_VERTEX = 16      # short Node(2) + pad(2) + float Position[3](12)
SIZEOF_NORMAL = 20      # short Node(2) + pad(2) + float Normal[3](12) + short BindVertex(2) + pad(2)
SIZEOF_TEXCOORD = 8     # float U(4) + float V(4)
SIZEOF_TRIANGLE = 64    # on-disk stride (Triangle_t2 with lightmap data)

# Encryption constants (from assets_convert.py)
MAP_XOR_KEY: Tuple[int, ...] = (
    0xD1, 0x73, 0x52, 0xF6, 0xD2, 0x9A, 0xCB, 0x27,
    0x3E, 0xAF, 0x59, 0x31, 0x37, 0xB3, 0xE7, 0xA2,
)
MAP_KEY_SEED = 0x5E

MAX_BONES = 200
MAX_MESH = 50

# Data-table BMD files that are NOT 3D models
NON_MODEL_STEMS = {
    "item", "minimap", "itemsetting", "petdata", "gate", "movereq",
    "npcname", "quest", "skill", "filter", "dialog", "movelist",
    "serverlist", "chaosbox", "mixlist",
}

# ---------------------------------------------------------------------------
# Decryption
# ---------------------------------------------------------------------------

def map_file_decrypt(data: bytes) -> bytes:
    out = bytearray(len(data))
    map_key = MAP_KEY_SEED
    key_len = len(MAP_XOR_KEY)
    for index, value in enumerate(data):
        out[index] = ((value ^ MAP_XOR_KEY[index % key_len]) - map_key) & 0xFF
        map_key = (value + 0x3D) & 0xFF
    return bytes(out)

# ---------------------------------------------------------------------------
# LEA-256 ECB Decryption (Season20 BMD version 0x0F)
# Port of KISA reference via xulek/muonline-bmd-viewer
# ---------------------------------------------------------------------------

_LEA_DELTA = (0xc3efe9db, 0x44626b02, 0x79e27c8a, 0x78df30ec,
              0x715ea49e, 0xc785da0a, 0xe04ef22a, 0xe5c40957)

_LEA_KEY = bytes([
    0xcc, 0x50, 0x45, 0x13, 0xc2, 0xa6, 0x57, 0x4e,
    0xd6, 0x9a, 0x45, 0x89, 0xbf, 0x2f, 0xbc, 0xd9,
    0x39, 0xb3, 0xb3, 0xbd, 0x50, 0xbd, 0xcc, 0xb6,
    0x85, 0x46, 0xd1, 0xd6, 0x16, 0x54, 0xe0, 0x87,
])

_M32 = 0xFFFFFFFF


def _rol32(x: int, n: int) -> int:
    n &= 31
    return ((x << n) | (x >> (32 - n))) & _M32


def _ror32(x: int, n: int) -> int:
    n &= 31
    return ((x >> n) | (x << (32 - n))) & _M32


def _lea256_key_schedule(key: bytes) -> List[int]:
    T = list(struct.unpack_from('<8I', key))
    rk = [0] * 192
    for i in range(32):
        d = _LEA_DELTA[i & 7]
        s = (i * 6) & 7
        T[(s+0)&7] = _rol32((T[(s+0)&7] + _rol32(d, i  )) & _M32,  1)
        T[(s+1)&7] = _rol32((T[(s+1)&7] + _rol32(d, i+1)) & _M32,  3)
        T[(s+2)&7] = _rol32((T[(s+2)&7] + _rol32(d, i+2)) & _M32,  6)
        T[(s+3)&7] = _rol32((T[(s+3)&7] + _rol32(d, i+3)) & _M32, 11)
        T[(s+4)&7] = _rol32((T[(s+4)&7] + _rol32(d, i+4)) & _M32, 13)
        T[(s+5)&7] = _rol32((T[(s+5)&7] + _rol32(d, i+5)) & _M32, 17)
        for j in range(6):
            rk[i * 6 + j] = T[(s + j) & 7]
    return rk


def _lea256_decrypt_block(block: Tuple[int, ...], rk: List[int]) -> Tuple[int, ...]:
    s = list(block)
    for r in range(31, -1, -1):
        base = r * 6
        t0 = s[3]
        t1 = (_ror32(s[0], 9) - (t0 ^ rk[base+0]) ^ rk[base+1]) & _M32
        t2 = (_rol32(s[1], 5) - (t1 ^ rk[base+2]) ^ rk[base+3]) & _M32
        t3 = (_rol32(s[2], 3) - (t2 ^ rk[base+4]) ^ rk[base+5]) & _M32
        s = [t0, t1, t2, t3]
    return tuple(s)


# Pre-compute round keys once at import time
_LEA_RK = _lea256_key_schedule(_LEA_KEY)


def lea256_ecb_decrypt(data: bytes) -> bytes:
    """Decrypt data using LEA-256 in ECB mode (16-byte block cipher)."""
    if len(data) % 16 != 0:
        raise BmdParseError(
            f"LEA-256 ECB payload must be 16-byte aligned (got {len(data)} bytes)"
        )

    out = bytearray(len(data))
    rk = _LEA_RK
    for off in range(0, len(data), 16):
        block = struct.unpack_from('<4I', data, off)
        dec = _lea256_decrypt_block(block, rk)
        struct.pack_into('<4I', out, off, *dec)
    return bytes(out)


def _find_modulus_tool() -> Optional[Path]:
    """Find the mu_terrain_decrypt tool (also works for BMD ModulusDecrypt)."""
    script_dir = Path(__file__).resolve().parent
    for name in ("mu_terrain_decrypt",):
        candidate = script_dir / name
        if candidate.is_file() and os.access(candidate, os.X_OK):
            return candidate
    return None


def _modulus_decrypt_bmd(enc_body: bytes) -> bytes:
    """Decrypt ModulusDecrypt-encrypted BMD body using the C++ tool.

    Wraps the encrypted body with a MAP\\x01 header so the tool processes it
    (MAP applies pure ModulusDecrypt with no Xor3Byte post-processing).
    """
    tool = _find_modulus_tool()
    if tool is None:
        raise BmdParseError(
            "mu_terrain_decrypt tool not found (needed for version 0x0E). "
            "Build it with: g++ -O2 -o mu_terrain_decrypt mu_terrain_decrypt.cpp -lcryptopp"
        )
    fake_data = b'MAP\x01' + enc_body
    with tempfile.NamedTemporaryFile(suffix=".bin", delete=False) as tmp_in:
        tmp_in.write(fake_data)
        tmp_in_path = tmp_in.name
    tmp_out_path = tmp_in_path + ".dec"
    try:
        result = subprocess.run(
            [str(tool), tmp_in_path, tmp_out_path],
            capture_output=True, text=True, timeout=30,
        )
        if result.returncode != 0:
            raise BmdParseError(
                f"ModulusDecrypt failed (rc={result.returncode}): {result.stderr.strip()}"
            )
        return Path(tmp_out_path).read_bytes()
    finally:
        Path(tmp_in_path).unlink(missing_ok=True)
        Path(tmp_out_path).unlink(missing_ok=True)

# ---------------------------------------------------------------------------
# Math helpers (ported from ZzzMathLib.cpp)
# ---------------------------------------------------------------------------

def angle_matrix(angles: Tuple[float, float, float]) -> List[List[float]]:
    """Compute a 3x4 rotation matrix from Euler angles in degrees (ZYX convention)."""
    a = angles[2] * (Q_PI * 2.0 / 360.0)
    sy, cy = math.sin(a), math.cos(a)
    a = angles[1] * (Q_PI * 2.0 / 360.0)
    sp, cp = math.sin(a), math.cos(a)
    a = angles[0] * (Q_PI * 2.0 / 360.0)
    sr, cr = math.sin(a), math.cos(a)

    return [
        [cp * cy,           sr * sp * cy + cr * (-sy), cr * sp * cy + (-sr) * (-sy), 0.0],
        [cp * sy,           sr * sp * sy + cr * cy,    cr * sp * sy + (-sr) * cy,     0.0],
        [-sp,               sr * cp,                   cr * cp,                        0.0],
    ]


def r_concat_transforms(
    in1: List[List[float]], in2: List[List[float]]
) -> List[List[float]]:
    """Concatenate two 3x4 transforms."""
    out = [[0.0] * 4 for _ in range(3)]
    for i in range(3):
        for j in range(4):
            s = in1[i][0] * in2[0][j] + in1[i][1] * in2[1][j] + in1[i][2] * in2[2][j]
            if j == 3:
                s += in1[i][3]
            out[i][j] = s
    return out


def vector_transform(v: Tuple[float, float, float], m: List[List[float]]) -> Tuple[float, float, float]:
    """Transform a vector by a 3x4 matrix (rotate + translate)."""
    return (
        v[0] * m[0][0] + v[1] * m[0][1] + v[2] * m[0][2] + m[0][3],
        v[0] * m[1][0] + v[1] * m[1][1] + v[2] * m[1][2] + m[1][3],
        v[0] * m[2][0] + v[1] * m[2][1] + v[2] * m[2][2] + m[2][3],
    )


def vector_rotate(v: Tuple[float, float, float], m: List[List[float]]) -> Tuple[float, float, float]:
    """Rotate a vector by the 3x3 part of a 3x4 matrix (no translation)."""
    return (
        v[0] * m[0][0] + v[1] * m[0][1] + v[2] * m[0][2],
        v[0] * m[1][0] + v[1] * m[1][1] + v[2] * m[1][2],
        v[0] * m[2][0] + v[1] * m[2][1] + v[2] * m[2][2],
    )


def vector_normalize(v: Tuple[float, float, float]) -> Tuple[float, float, float]:
    length = math.sqrt(v[0] * v[0] + v[1] * v[1] + v[2] * v[2])
    if length == 0.0:
        return (0.0, 0.0, 0.0)
    return (v[0] / length, v[1] / length, v[2] / length)


# ---------------------------------------------------------------------------
# BMD data structures
# ---------------------------------------------------------------------------

@dataclass
class BmdVertex:
    node: int
    position: Tuple[float, float, float]


@dataclass
class BmdNormal:
    node: int
    normal: Tuple[float, float, float]
    bind_vertex: int


@dataclass
class BmdTexCoord:
    u: float
    v: float


@dataclass
class BmdTriangle:
    polygon: int
    vertex_index: Tuple[int, int, int, int]
    normal_index: Tuple[int, int, int, int]
    texcoord_index: Tuple[int, int, int, int]


@dataclass
class BmdMesh:
    num_vertices: int
    num_normals: int
    num_texcoords: int
    num_triangles: int
    texture: int
    vertices: List[BmdVertex]
    normals: List[BmdNormal]
    texcoords: List[BmdTexCoord]
    triangles: List[BmdTriangle]
    texture_name: str


@dataclass
class BmdAction:
    num_animation_keys: int
    lock_positions: bool
    positions: Optional[List[Tuple[float, float, float]]]


@dataclass
class BmdBone:
    name: str
    parent: int
    dummy: bool
    # Per-action: list of (positions_per_key, rotations_per_key)
    matrices: List[Tuple[List[Tuple[float, float, float]], List[Tuple[float, float, float]]]]


@dataclass
class BoneFixup:
    m: List[List[float]]
    world_org: Tuple[float, float, float]


@dataclass
class BmdModel:
    name: str
    version: int
    num_meshs: int
    num_bones: int
    num_actions: int
    meshs: List[BmdMesh]
    actions: List[BmdAction]
    bones: List[BmdBone]


# ---------------------------------------------------------------------------
# BMD Parser
# ---------------------------------------------------------------------------


def _read_c_string(data: bytes, offset: int, length: int) -> str:
    raw = data[offset:offset + length]
    null_pos = raw.find(b'\x00')
    if null_pos >= 0:
        raw = raw[:null_pos]
    return raw.decode('ascii', errors='replace')


def parse_bmd(file_path: Path) -> BmdModel:
    """Parse a BMD file and return its model data."""
    raw = file_path.read_bytes()

    if len(raw) < 4:
        raise BmdParseError(f"File too small: {len(raw)} bytes")

    magic = raw[:3]
    if magic != b'BMD':
        raise BmdParseError(f"Not a BMD file (magic: {magic!r})")

    version = raw[3]

    if version == 0x00:
        raise BmdParseError("Corrupt/empty BMD (version 0x00)")

    if version == 0x0C:
        # MapFileDecrypt encrypted
        if len(raw) < 8:
            raise BmdParseError("Encrypted BMD too small for size header")
        enc_size = struct.unpack_from('<I', raw, 4)[0]
        if len(raw) < 8 + enc_size:
            raise BmdParseError(
                f"Encrypted BMD truncated (need {8 + enc_size}, have {len(raw)})"
            )
        data = map_file_decrypt(raw[8:8 + enc_size])
    elif version == 0x0A:
        # Unencrypted
        data = raw[4:]
    elif version == 0x0E:
        # Modulus encrypted (Season16+)
        if len(raw) < 8:
            raise BmdParseError("Encrypted BMD too small for size header")
        enc_size = struct.unpack_from('<I', raw, 4)[0]
        if len(raw) < 8 + enc_size:
            raise BmdParseError(
                f"Encrypted BMD truncated (need {8 + enc_size}, have {len(raw)})"
            )
        data = _modulus_decrypt_bmd(raw[8:8 + enc_size])
    elif version == 0x0F:
        # LEA-256 ECB encrypted (Season20)
        if len(raw) < 8:
            raise BmdParseError("Encrypted BMD too small for size header")
        enc_size = struct.unpack_from('<I', raw, 4)[0]
        if len(raw) < 8 + enc_size:
            raise BmdParseError(
                f"Encrypted BMD truncated (need {8 + enc_size}, have {len(raw)})"
            )
        if enc_size % 16 != 0:
            raise BmdParseError(
                f"LEA-256 encrypted BMD size must be 16-byte aligned (got {enc_size})"
            )
        data = lea256_ecb_decrypt(raw[8:8 + enc_size])
    else:
        raise BmdParseError(f"Unknown BMD version: 0x{version:02X}")

    if len(data) < 38:
        raise BmdParseError(f"BMD data too small for model header ({len(data)} < 38)")

    pos = 0

    # Model header: Name(32) + NumMeshs(2) + NumBones(2) + NumActions(2) = 38 bytes
    name = _read_c_string(data, pos, 32)
    pos += 32

    num_meshs = struct.unpack_from('<h', data, pos)[0]
    pos += 2
    num_bones = struct.unpack_from('<h', data, pos)[0]
    pos += 2
    num_actions = struct.unpack_from('<h', data, pos)[0]
    pos += 2

    if num_meshs < 0 or num_meshs > MAX_MESH:
        raise BmdParseError(f"Invalid NumMeshs: {num_meshs}")
    if num_bones < 0 or num_bones > MAX_BONES:
        raise BmdParseError(f"Invalid NumBones: {num_bones}")
    if num_actions < 0:
        raise BmdParseError(f"Invalid NumActions: {num_actions}")

    # Parse meshes
    meshs: List[BmdMesh] = []
    for i in range(num_meshs):
        if pos + 10 > len(data):
            raise BmdParseError(f"Mesh {i} header truncated at offset {pos}")

        nv = struct.unpack_from('<h', data, pos)[0]; pos += 2
        nn = struct.unpack_from('<h', data, pos)[0]; pos += 2
        ntc = struct.unpack_from('<h', data, pos)[0]; pos += 2
        nt = struct.unpack_from('<h', data, pos)[0]; pos += 2
        tex = struct.unpack_from('<h', data, pos)[0]; pos += 2

        if nv < 0: nv = 0
        if nn < 0: nn = 0
        if ntc < 0: ntc = 0
        if nt < 0: nt = 0

        # Vertices: N * 16 bytes each
        verts: List[BmdVertex] = []
        needed = nv * SIZEOF_VERTEX
        if pos + needed > len(data):
            raise BmdParseError(f"Mesh {i} vertices truncated")
        for j in range(nv):
            off = pos + j * SIZEOF_VERTEX
            node = struct.unpack_from('<h', data, off)[0]
            px, py, pz = struct.unpack_from('<3f', data, off + 4)
            verts.append(BmdVertex(node=node, position=(px, py, pz)))
        pos += needed

        # Normals: N * 20 bytes each
        norms: List[BmdNormal] = []
        needed = nn * SIZEOF_NORMAL
        if pos + needed > len(data):
            raise BmdParseError(f"Mesh {i} normals truncated")
        for j in range(nn):
            off = pos + j * SIZEOF_NORMAL
            node = struct.unpack_from('<h', data, off)[0]
            nx, ny, nz = struct.unpack_from('<3f', data, off + 4)
            bv = struct.unpack_from('<h', data, off + 16)[0]
            norms.append(BmdNormal(node=node, normal=(nx, ny, nz), bind_vertex=bv))
        pos += needed

        # TexCoords: N * 8 bytes each
        tcs: List[BmdTexCoord] = []
        needed = ntc * SIZEOF_TEXCOORD
        if pos + needed > len(data):
            raise BmdParseError(f"Mesh {i} texcoords truncated")
        for j in range(ntc):
            off = pos + j * SIZEOF_TEXCOORD
            u, v = struct.unpack_from('<2f', data, off)
            tcs.append(BmdTexCoord(u=u, v=v))
        pos += needed

        # Triangles: N * 64 bytes stride each (on-disk Triangle_t2)
        tris: List[BmdTriangle] = []
        needed = nt * SIZEOF_TRIANGLE
        if pos + needed > len(data):
            raise BmdParseError(f"Mesh {i} triangles truncated")
        for j in range(nt):
            off = pos + j * SIZEOF_TRIANGLE
            polygon = struct.unpack_from('<b', data, off)[0]
            # skip 1 byte padding
            vi = struct.unpack_from('<4h', data, off + 2)
            ni = struct.unpack_from('<4h', data, off + 10)
            ti = struct.unpack_from('<4h', data, off + 18)
            tris.append(BmdTriangle(
                polygon=polygon,
                vertex_index=vi,
                normal_index=ni,
                texcoord_index=ti,
            ))
        pos += needed

        # Texture name: 32 bytes
        if pos + 32 > len(data):
            raise BmdParseError(f"Mesh {i} texture name truncated")
        tex_name = _read_c_string(data, pos, 32)
        pos += 32

        meshs.append(BmdMesh(
            num_vertices=nv, num_normals=nn, num_texcoords=ntc,
            num_triangles=nt, texture=tex,
            vertices=verts, normals=norms, texcoords=tcs, triangles=tris,
            texture_name=tex_name,
        ))

    # Parse actions
    actions: List[BmdAction] = []
    for i in range(num_actions):
        if pos + 3 > len(data):
            raise BmdParseError(f"Action {i} header truncated")

        num_keys = struct.unpack_from('<h', data, pos)[0]; pos += 2
        lock_pos = struct.unpack_from('<B', data, pos)[0]; pos += 1
        if num_keys < 0:
            num_keys = 0

        positions = None
        if lock_pos:
            needed = num_keys * 12  # vec3_t = 3 floats
            if pos + needed > len(data):
                raise BmdParseError(f"Action {i} positions truncated")
            positions = []
            for j in range(num_keys):
                off = pos + j * 12
                px, py, pz = struct.unpack_from('<3f', data, off)
                positions.append((px, py, pz))
            pos += needed

        actions.append(BmdAction(
            num_animation_keys=num_keys,
            lock_positions=bool(lock_pos),
            positions=positions,
        ))

    # Parse bones
    bones: List[BmdBone] = []
    for i in range(num_bones):
        if pos + 1 > len(data):
            raise BmdParseError(f"Bone {i} header truncated")

        dummy = struct.unpack_from('<b', data, pos)[0]; pos += 1

        if not dummy:
            if pos + 34 > len(data):
                raise BmdParseError(f"Bone {i} data truncated")

            bone_name = _read_c_string(data, pos, 32); pos += 32
            parent = struct.unpack_from('<h', data, pos)[0]; pos += 2

            matrices: List[Tuple[List[Tuple[float, float, float]], List[Tuple[float, float, float]]]] = []
            for j in range(num_actions):
                nkeys = actions[j].num_animation_keys
                needed = nkeys * 12 * 2  # position + rotation
                if pos + needed > len(data):
                    raise BmdParseError(f"Bone {i} action {j} data truncated")

                bone_positions = []
                for k in range(nkeys):
                    off = pos + k * 12
                    px, py, pz = struct.unpack_from('<3f', data, off)
                    bone_positions.append((px, py, pz))
                pos += nkeys * 12

                bone_rotations = []
                for k in range(nkeys):
                    off = pos + k * 12
                    rx, ry, rz = struct.unpack_from('<3f', data, off)
                    bone_rotations.append((rx, ry, rz))
                pos += nkeys * 12

                matrices.append((bone_positions, bone_rotations))

            bones.append(BmdBone(
                name=bone_name, parent=parent, dummy=False, matrices=matrices,
            ))
        else:
            # Dummy bone
            bones.append(BmdBone(
                name=f"Null_{i}", parent=-1, dummy=True,
                matrices=[([( 0.0, 0.0, 0.0)], [(0.0, 0.0, 0.0)])]
            ))

    return BmdModel(
        name=name, version=version,
        num_meshs=num_meshs, num_bones=num_bones, num_actions=num_actions,
        meshs=meshs, actions=actions, bones=bones,
    )


# ---------------------------------------------------------------------------
# Bone Fixup (rest pose world transforms)
# ---------------------------------------------------------------------------

def compute_bone_fixups(model: BmdModel) -> List[BoneFixup]:
    """Compute world-space transforms for each bone at rest pose (action=0, key=0).

    Reference: BMD_SMD.cpp:156-195 (FixUpBones)
    """
    fixups: List[BoneFixup] = []
    identity_m = [[1, 0, 0, 0], [0, 1, 0, 0], [0, 0, 1, 0]]

    for i in range(model.num_bones):
        bone = model.bones[i]

        if bone.dummy or not bone.matrices:
            fixups.append(BoneFixup(m=identity_m, world_org=(0.0, 0.0, 0.0)))
            continue

        # Get rotation and position from action=0, key=0
        positions_0, rotations_0 = bone.matrices[0]
        rot = rotations_0[0] if rotations_0 else (0.0, 0.0, 0.0)
        bpos = positions_0[0] if positions_0 else (0.0, 0.0, 0.0)

        # Convert rotation from radians to degrees (BMD stores radians)
        # Reference: BMD_SMD.cpp:165-167 — Angle = Rotation * (180/PI)
        angle_deg = (
            rot[0] * (180.0 / Q_PI),
            rot[1] * (180.0 / Q_PI),
            rot[2] * (180.0 / Q_PI),
        )

        if bone.parent >= 0 and bone.parent < len(fixups):
            local_m = angle_matrix(angle_deg)
            parent_fixup = fixups[bone.parent]
            world_m = r_concat_transforms(parent_fixup.m, local_m)
            p = vector_transform(bpos, parent_fixup.m)
            world_org = (
                p[0] + parent_fixup.world_org[0],
                p[1] + parent_fixup.world_org[1],
                p[2] + parent_fixup.world_org[2],
            )
            fixups.append(BoneFixup(m=world_m, world_org=world_org))
        else:
            m = angle_matrix(angle_deg)
            fixups.append(BoneFixup(m=m, world_org=bpos))

    return fixups


# ---------------------------------------------------------------------------
# GLTF / GLB Emission
# ---------------------------------------------------------------------------

def bmd_to_glb(model: BmdModel) -> Optional[bytes]:
    """Convert a parsed BMD model to GLB (GLTF Binary) bytes.

    Returns None if the model has no renderable geometry.
    """
    if model.num_meshs == 0:
        return None

    # Check if there are any triangles at all
    total_tris = sum(m.num_triangles for m in model.meshs)
    if total_tris == 0:
        return None

    # Compute bone fixups for world-space transform
    if model.num_bones > 0 and model.num_actions > 0:
        fixups = compute_bone_fixups(model)
    else:
        fixups = []

    # Build unified vertex buffer per mesh, then combine into GLTF primitives
    all_positions: List[Tuple[float, float, float]] = []
    all_normals: List[Tuple[float, float, float]] = []
    all_texcoords: List[Tuple[float, float]] = []
    all_indices: List[int] = []

    primitives_info: List[Tuple[int, int, int, int]] = []  # (vert_offset, vert_count, idx_offset, idx_count)

    for mesh in model.meshs:
        if mesh.num_triangles == 0:
            continue

        # De-index: build combined vertices
        vert_map: Dict[Tuple[int, int, int], int] = {}
        mesh_positions: List[Tuple[float, float, float]] = []
        mesh_normals: List[Tuple[float, float, float]] = []
        mesh_texcoords: List[Tuple[float, float]] = []
        mesh_indices: List[int] = []

        for tri in mesh.triangles:
            n_corners = min(tri.polygon, 4) if tri.polygon >= 3 else 3

            # Collect corner data
            corners: List[int] = []
            for k in range(n_corners):
                vi = tri.vertex_index[k]
                ni = tri.normal_index[k]
                ti = tri.texcoord_index[k]

                # Bounds check
                if vi < 0 or vi >= mesh.num_vertices:
                    continue
                if ni < 0 or ni >= mesh.num_normals:
                    continue
                if ti < 0 or ti >= mesh.num_texcoords:
                    continue

                key = (vi, ni, ti)
                if key in vert_map:
                    corners.append(vert_map[key])
                else:
                    idx = len(mesh_positions)
                    vert_map[key] = idx

                    vert = mesh.vertices[vi]
                    norm = mesh.normals[ni]
                    tc = mesh.texcoords[ti]

                    # Transform vertex to world space using bone fixup
                    vnode = vert.node
                    nnode = norm.node

                    if fixups and 0 <= vnode < len(fixups):
                        wp = vector_transform(vert.position, fixups[vnode].m)
                        world_pos = (
                            wp[0] + fixups[vnode].world_org[0],
                            wp[1] + fixups[vnode].world_org[1],
                            wp[2] + fixups[vnode].world_org[2],
                        )
                    elif fixups and vnode >= len(fixups):
                        # Clamp to bone 0
                        logging.warning(
                            "Vertex node %d >= num_bones %d in %s, clamping to 0",
                            vnode, len(fixups), model.name,
                        )
                        wp = vector_transform(vert.position, fixups[0].m)
                        world_pos = (
                            wp[0] + fixups[0].world_org[0],
                            wp[1] + fixups[0].world_org[1],
                            wp[2] + fixups[0].world_org[2],
                        )
                    else:
                        world_pos = vert.position

                    if fixups and 0 <= nnode < len(fixups):
                        wn = vector_rotate(norm.normal, fixups[nnode].m)
                        world_norm = vector_normalize(wn)
                    elif fixups and nnode >= len(fixups):
                        wn = vector_rotate(norm.normal, fixups[0].m)
                        world_norm = vector_normalize(wn)
                    else:
                        world_norm = vector_normalize(norm.normal)

                    mesh_positions.append(world_pos)
                    mesh_normals.append(world_norm)
                    # UV flip: v_gltf = 1.0 - v_bmd (reference: BMD_SMD.cpp:755)
                    mesh_texcoords.append((tc.u, 1.0 - tc.v))
                    corners.append(idx)

            # Triangulate
            if len(corners) >= 3:
                mesh_indices.extend([corners[0], corners[1], corners[2]])
            if len(corners) >= 4:
                # Quad -> two triangles: (0,1,2) and (0,2,3)
                mesh_indices.extend([corners[0], corners[2], corners[3]])

        if not mesh_indices:
            continue

        vert_offset = len(all_positions)
        idx_offset = len(all_indices)

        all_positions.extend(mesh_positions)
        all_normals.extend(mesh_normals)
        all_texcoords.extend(mesh_texcoords)
        # Offset indices
        all_indices.extend(i + vert_offset for i in mesh_indices)

        primitives_info.append((
            vert_offset, len(mesh_positions),
            idx_offset, len(mesh_indices),
        ))

    if not all_positions or not all_indices:
        return None

    num_verts = len(all_positions)
    num_indices = len(all_indices)
    use_uint32 = num_verts > 65535

    # Compute bounding box for POSITION accessor
    min_pos = [float('inf')] * 3
    max_pos = [float('-inf')] * 3
    for p in all_positions:
        for c in range(3):
            if p[c] < min_pos[c]:
                min_pos[c] = p[c]
            if p[c] > max_pos[c]:
                max_pos[c] = p[c]

    # Build binary buffer
    pos_data = b''.join(struct.pack('<3f', *p) for p in all_positions)
    norm_data = b''.join(struct.pack('<3f', *n) for n in all_normals)
    tc_data = b''.join(struct.pack('<2f', *t) for t in all_texcoords)
    if use_uint32:
        idx_data = b''.join(struct.pack('<I', i) for i in all_indices)
    else:
        idx_data = b''.join(struct.pack('<H', i) for i in all_indices)

    pos_offset = 0
    pos_size = len(pos_data)
    norm_offset = pos_offset + pos_size
    norm_size = len(norm_data)
    tc_offset = norm_offset + norm_size
    tc_size = len(tc_data)
    idx_offset_buf = tc_offset + tc_size
    idx_size = len(idx_data)

    total_buf = pos_size + norm_size + tc_size + idx_size

    binary_buffer = pos_data + norm_data + tc_data + idx_data

    # Build GLTF JSON
    gltf = {
        "asset": {"version": "2.0", "generator": "mu-rust bmd_converter.py"},
        "scene": 0,
        "scenes": [{"nodes": [0]}],
        "nodes": [{"mesh": 0, "name": model.name}],
        "buffers": [{"byteLength": total_buf}],
        "bufferViews": [
            # 0: positions
            {"buffer": 0, "byteOffset": pos_offset, "byteLength": pos_size, "target": 34962},
            # 1: normals
            {"buffer": 0, "byteOffset": norm_offset, "byteLength": norm_size, "target": 34962},
            # 2: texcoords
            {"buffer": 0, "byteOffset": tc_offset, "byteLength": tc_size, "target": 34962},
            # 3: indices
            {"buffer": 0, "byteOffset": idx_offset_buf, "byteLength": idx_size, "target": 34963},
        ],
        "accessors": [
            # 0: POSITION
            {
                "bufferView": 0, "componentType": 5126, "count": num_verts,
                "type": "VEC3", "max": max_pos, "min": min_pos,
            },
            # 1: NORMAL
            {
                "bufferView": 1, "componentType": 5126, "count": num_verts,
                "type": "VEC3",
            },
            # 2: TEXCOORD_0
            {
                "bufferView": 2, "componentType": 5126, "count": num_verts,
                "type": "VEC2",
            },
            # 3: indices
            {
                "bufferView": 3,
                "componentType": 5125 if use_uint32 else 5123,
                "count": num_indices,
                "type": "SCALAR",
            },
        ],
        "meshes": [{
            "name": model.name,
            "primitives": [],
        }],
    }

    # Build primitives (one per mesh, or combined)
    if len(primitives_info) == 1:
        gltf["meshes"][0]["primitives"] = [{
            "attributes": {
                "POSITION": 0,
                "NORMAL": 1,
                "TEXCOORD_0": 2,
            },
            "indices": 3,
        }]
    else:
        # Multiple meshes: we've already combined into a single buffer,
        # but we use a single primitive for simplicity. Per-mesh primitives
        # would need separate accessors per mesh. For the Bevy client,
        # a single combined primitive works fine.
        gltf["meshes"][0]["primitives"] = [{
            "attributes": {
                "POSITION": 0,
                "NORMAL": 1,
                "TEXCOORD_0": 2,
            },
            "indices": 3,
        }]

    # Encode GLB
    json_bytes = json.dumps(gltf, indent=2).encode('ascii')
    # Pad JSON to 4-byte alignment with spaces
    json_pad = (4 - len(json_bytes) % 4) % 4
    json_bytes += b' ' * json_pad

    # Pad binary buffer to 4-byte alignment with zeros
    bin_pad = (4 - len(binary_buffer) % 4) % 4
    binary_buffer += b'\x00' * bin_pad

    # GLB header
    total_length = 12 + 8 + len(json_bytes) + 8 + len(binary_buffer)
    glb = bytearray()
    # Header: magic + version + length
    glb += struct.pack('<III', 0x46546C67, 2, total_length)  # "glTF", version 2
    # JSON chunk
    glb += struct.pack('<II', len(json_bytes), 0x4E4F534A)  # "JSON"
    glb += json_bytes
    # BIN chunk
    glb += struct.pack('<II', len(binary_buffer), 0x004E4942)  # "BIN\0"
    glb += binary_buffer

    return bytes(glb)


# ---------------------------------------------------------------------------
# Batch conversion
# ---------------------------------------------------------------------------

@dataclass
class ConversionStats:
    total_found: int = 0
    converted: int = 0
    skipped_no_geometry: int = 0
    skipped_non_model: int = 0
    skipped_existing: int = 0
    skipped_corrupt: int = 0
    failed: int = 0
    failures: List[Dict] = field(default_factory=list)


def is_non_model_bmd(file_path: Path) -> bool:
    """Check if a BMD file is actually a data table, not a 3D model."""
    stem = file_path.stem.lower()
    return stem in NON_MODEL_STEMS


def convert_single_bmd(
    source: Path,
    output_path: Path,
    force: bool,
    stats: ConversionStats,
) -> None:
    """Convert a single BMD file to GLB."""
    stats.total_found += 1

    if is_non_model_bmd(source):
        stats.skipped_non_model += 1
        logging.debug("Skipping non-model BMD: %s", source)
        return

    # Check magic without full parse
    try:
        with open(source, 'rb') as f:
            header = f.read(4)
        if len(header) < 3 or header[:3] != b'BMD':
            stats.skipped_non_model += 1
            logging.debug("Skipping non-BMD file: %s (magic: %r)", source, header[:3])
            return
    except OSError as exc:
        stats.failed += 1
        stats.failures.append({"source": str(source), "error": str(exc)})
        logging.error("Cannot read %s: %s", source, exc)
        return

    if not force and output_path.exists() and output_path.stat().st_size >= 128:
        stats.skipped_existing += 1
        logging.debug("Skipping existing: %s", output_path)
        return

    try:
        model = parse_bmd(source)
    except BmdParseError as exc:
        stats.skipped_corrupt += 1
        stats.failures.append({"source": str(source), "error": str(exc), "type": "parse"})
        logging.warning("Parse error for %s: %s", source, exc)
        return
    except Exception as exc:
        stats.failed += 1
        stats.failures.append({"source": str(source), "error": str(exc), "type": "unexpected"})
        logging.error("Unexpected error parsing %s: %s", source, exc)
        return

    try:
        glb_bytes = bmd_to_glb(model)
    except Exception as exc:
        stats.failed += 1
        stats.failures.append({"source": str(source), "error": str(exc), "type": "convert"})
        logging.error("Conversion error for %s: %s", source, exc)
        return

    if glb_bytes is None:
        stats.skipped_no_geometry += 1
        logging.debug("No geometry in %s (meshs=%d)", source, model.num_meshs)
        return

    if len(glb_bytes) < 128:
        stats.failed += 1
        stats.failures.append({
            "source": str(source), "error": f"GLB too small ({len(glb_bytes)} bytes)",
            "type": "validation",
        })
        logging.warning("GLB output too small for %s: %d bytes", source, len(glb_bytes))
        return

    output_path.parent.mkdir(parents=True, exist_ok=True)
    output_path.write_bytes(glb_bytes)
    stats.converted += 1
    logging.debug("Converted %s -> %s (%d bytes)", source, output_path, len(glb_bytes))


def discover_bmd_files(root: Path) -> List[Path]:
    """Discover all .bmd files under root, case-insensitive."""
    result = []
    for dirpath, _dirnames, filenames in os.walk(root):
        for fname in filenames:
            if fname.lower().endswith('.bmd'):
                result.append(Path(dirpath) / fname)
    result.sort()
    return result


def convert_all(
    bmd_root: Path,
    output_root: Path,
    fmt: str,
    force: bool,
    dry_run: bool,
    verbose: bool,
    report_path: Optional[Path],
) -> ConversionStats:
    """Convert all BMD files found under bmd_root."""
    stats = ConversionStats()

    bmd_files = discover_bmd_files(bmd_root)
    total = len(bmd_files)
    logging.info("Found %d BMD files under %s", total, bmd_root)

    if dry_run:
        for f in bmd_files:
            rel = f.relative_to(bmd_root)
            out = output_root / rel.with_suffix('.glb')
            logging.info("[DRY-RUN] Would convert %s -> %s", f, out)
        stats.total_found = total
        return stats

    start_time = time.time()
    for idx, bmd_path in enumerate(bmd_files):
        rel = bmd_path.relative_to(bmd_root)
        out_path = output_root / rel.with_suffix('.glb')

        convert_single_bmd(bmd_path, out_path, force, stats)

        if (idx + 1) % 500 == 0 or (idx + 1) == total:
            elapsed = time.time() - start_time
            logging.info(
                "Progress: %d/%d (%.1f%%) — converted=%d skipped=%d failed=%d [%.1fs]",
                idx + 1, total, 100.0 * (idx + 1) / total,
                stats.converted,
                stats.skipped_no_geometry + stats.skipped_non_model + stats.skipped_existing + stats.skipped_corrupt,
                stats.failed,
                elapsed,
            )

    elapsed = time.time() - start_time
    logging.info(
        "Conversion complete in %.1fs: %d converted, %d skipped (no_geom=%d, non_model=%d, existing=%d, corrupt=%d), %d failed",
        elapsed, stats.converted,
        stats.skipped_no_geometry + stats.skipped_non_model + stats.skipped_existing + stats.skipped_corrupt,
        stats.skipped_no_geometry, stats.skipped_non_model, stats.skipped_existing, stats.skipped_corrupt,
        stats.failed,
    )

    if report_path:
        report_path.parent.mkdir(parents=True, exist_ok=True)
        report = {
            "total_found": stats.total_found,
            "converted": stats.converted,
            "skipped_no_geometry": stats.skipped_no_geometry,
            "skipped_non_model": stats.skipped_non_model,
            "skipped_existing": stats.skipped_existing,
            "skipped_corrupt": stats.skipped_corrupt,
            "failed": stats.failed,
            "failures": stats.failures,
        }
        report_path.write_text(json.dumps(report, indent=2))
        logging.info("Report written to %s", report_path)

    return stats


# ---------------------------------------------------------------------------
# CLI
# ---------------------------------------------------------------------------

def main() -> int:
    parser = argparse.ArgumentParser(
        description="Convert MU Online BMD 3D model files to GLTF Binary (GLB)."
    )
    parser.add_argument(
        "--bmd-root", type=Path, required=True,
        help="Root directory containing legacy BMD files",
    )
    parser.add_argument(
        "--output-root", type=Path, required=True,
        help="Output directory for converted GLB files",
    )
    parser.add_argument(
        "--format", choices=["glb"], default="glb",
        help="Output format (default: glb)",
    )
    parser.add_argument("--force", action="store_true", help="Force reconversion")
    parser.add_argument("--dry-run", action="store_true", help="Show what would be done")
    parser.add_argument("--verbose", action="store_true", help="Enable verbose logging")
    parser.add_argument(
        "--report", type=Path, default=None,
        help="Path for JSON conversion report",
    )

    args = parser.parse_args()

    logging.basicConfig(
        level=logging.DEBUG if args.verbose else logging.INFO,
        format="%(asctime)s %(levelname)-8s %(message)s",
        datefmt="%H:%M:%S",
    )

    if not args.bmd_root.is_dir():
        logging.error("BMD root directory not found: %s", args.bmd_root)
        return 1

    stats = convert_all(
        bmd_root=args.bmd_root,
        output_root=args.output_root,
        fmt=args.format,
        force=args.force,
        dry_run=args.dry_run,
        verbose=args.verbose,
        report_path=args.report,
    )

    if stats.failed > 0:
        logging.warning("%d files failed conversion", stats.failed)

    return 0


if __name__ == "__main__":
    sys.exit(main())
