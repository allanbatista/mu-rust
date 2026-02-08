# MU Online Client Asset Formats

Reference documentation for the legacy MU Online client asset formats and the
conversion pipeline that produces Bevy-compatible assets (GLB, PNG, JSON).

---

## Table of Contents

1. [Pipeline Overview](#1-pipeline-overview)
2. [3D Models (BMD)](#2-3d-models-bmd)
3. [Textures](#3-textures)
4. [Terrain Height Map](#4-terrain-height-map)
5. [Terrain Tile Mapping](#5-terrain-tile-mapping)
6. [Terrain Attributes](#6-terrain-attributes)
7. [Scene Objects](#7-scene-objects)
8. [Camera Tour](#8-camera-tour)
9. [Terrain Config](#9-terrain-config)
10. [Encryption Algorithms](#10-encryption-algorithms)
11. [Terrain Constants](#11-terrain-constants)
12. [Extension Catalog](#12-extension-catalog)

---

## 1. Pipeline Overview

The conversion pipeline reads legacy MU Online client data and produces assets
consumable by the Rust/Bevy client.

```
Legacy Data/                        Converted Assets/
  *.bmd  ─── bmd_converter.py ───>    *.glb          (3D models)
  *.ozj  ┐                            *.png          (textures)
  *.ozt  │── assets_convert.py ──>
  *.ozb  │                             terrain_height.json
  *.ozp  │                             *.map.json
  *.tga  │                             *.att.json  (or EncTerrainN.json)
  *.bmp  │                             scene_objects.json
  *.jpg  ┘                             camera_tour.json
  *.map  ─── assets_convert.py ──>     terrain_config.json
  *.att  ─── assets_convert.py ──>
  *.obj  ─── assets_convert.py ──>
  *.cws  ─── assets_convert.py ──>
```

**Asset categories:**

| Category       | Source Formats             | Output Format | Converter               |
|----------------|----------------------------|---------------|-------------------------|
| 3D Models      | `.bmd`                     | `.glb`        | `bmd_converter.py`      |
| Textures       | `.ozj` `.ozt` `.ozb` `.ozp` `.tga` `.bmp` `.jpg` `.png` | `.png` | `assets_convert.py` |
| Height Map     | `TerrainHeight.OZB`       | `terrain_height.json` | `assets_convert.py` |
| Tile Map       | `EncTerrainN.map` / `TerrainN.map` | `*.map.json` | `assets_convert.py` |
| Attributes     | `EncTerrainN.att` / `TerrainN.att` | `*.att.json` | `assets_convert.py` |
| Scene Objects  | `EncTerrainN.obj`          | `scene_objects.json` | `assets_convert.py` |
| Camera Tour    | `CWScriptN.cws`           | `camera_tour.json`   | `assets_convert.py` |
| Terrain Config | (generated placeholder)    | `terrain_config.json`| `assets_convert.py` |

---

## 2. 3D Models (BMD)

Source: `bmd_converter.py`
Reference: `ZzzBMD.cpp:2530-2625` (Open/Open2), `BMD_SMD.cpp:156-195` (FixUpBones)

### 2.1 File Header

Every BMD file starts with a 4-byte header:

| Offset | Size | Field   | Description                          |
|--------|------|---------|--------------------------------------|
| 0      | 3    | Magic   | ASCII `BMD`                          |
| 3      | 1    | Version | Encryption/format version byte       |

### 2.2 Version Variants

| Version | Encryption     | Era       | Data Offset | Notes                                    |
|---------|----------------|-----------|-------------|------------------------------------------|
| `0x0A`  | None           | Legacy    | 4           | Plaintext BMD data starts immediately.   |
| `0x0C`  | MapFileDecrypt | v5.x      | 8           | 4-byte LE encrypted size at offset 4.   |
| `0x0E`  | ModulusDecrypt | Season16+ | 8           | Same header layout as 0x0C.             |
| `0x0F`  | LEA-256 ECB    | Season20  | 8           | Same header layout as 0x0C. Size must be 16-byte aligned. |

For versions `0x0C`, `0x0E`, `0x0F`:

| Offset | Size | Field        | Description                            |
|--------|------|--------------|----------------------------------------|
| 4      | 4    | EncryptedSize| LE uint32 — byte count of encrypted body |
| 8      | N    | Body         | Encrypted payload (N = EncryptedSize)  |

### 2.3 Decrypted Data Layout

After decryption the binary payload has:

#### Model Header (38 bytes)

| Offset | Size | Type     | Field       | Description                     |
|--------|------|----------|-------------|---------------------------------|
| 0      | 32   | char[32] | Name        | Null-terminated ASCII model name |
| 32     | 2    | int16    | NumMeshs    | Number of meshes (max 50)       |
| 34     | 2    | int16    | NumBones    | Number of bones (max 200)       |
| 36     | 2    | int16    | NumActions  | Number of animation actions     |

#### Mesh Array (repeated NumMeshs times)

Each mesh starts with a 10-byte sub-header, followed by variable-length data:

**Mesh sub-header (10 bytes):**

| Offset | Size | Type  | Field        |
|--------|------|-------|--------------|
| 0      | 2    | int16 | NumVertices  |
| 2      | 2    | int16 | NumNormals   |
| 4      | 2    | int16 | NumTexCoords |
| 6      | 2    | int16 | NumTriangles |
| 8      | 2    | int16 | TextureIndex |

**Vertex_t (16 bytes each, repeated NumVertices times):**

| Offset | Size | Type    | Field       | Description               |
|--------|------|---------|-------------|---------------------------|
| 0      | 2    | int16   | Node        | Bone index                |
| 2      | 2    | —       | (padding)   | Alignment padding         |
| 4      | 4    | float32 | Position.X  |                           |
| 8      | 4    | float32 | Position.Y  |                           |
| 12     | 4    | float32 | Position.Z  |                           |

**Normal_t (20 bytes each, repeated NumNormals times):**

| Offset | Size | Type    | Field       | Description               |
|--------|------|---------|-------------|---------------------------|
| 0      | 2    | int16   | Node        | Bone index                |
| 2      | 2    | —       | (padding)   | Alignment padding         |
| 4      | 4    | float32 | Normal.X    |                           |
| 8      | 4    | float32 | Normal.Y    |                           |
| 12     | 4    | float32 | Normal.Z    |                           |
| 16     | 2    | int16   | BindVertex  | Associated vertex index   |
| 18     | 2    | —       | (padding)   | Alignment padding         |

**TexCoord_t (8 bytes each, repeated NumTexCoords times):**

| Offset | Size | Type    | Field |
|--------|------|---------|-------|
| 0      | 4    | float32 | U     |
| 4      | 4    | float32 | V     |

**Triangle_t2 (64 bytes stride each, repeated NumTriangles times):**

| Offset | Size | Type     | Field           | Description                          |
|--------|------|----------|-----------------|--------------------------------------|
| 0      | 1    | int8     | Polygon         | Vertex count per face (3 or 4)       |
| 1      | 1    | —        | (padding)       |                                      |
| 2      | 8    | int16[4] | VertexIndex     | Indices into vertex array            |
| 10     | 8    | int16[4] | NormalIndex     | Indices into normal array            |
| 18     | 8    | int16[4] | TexCoordIndex   | Indices into texcoord array          |
| 26     | 38   | —        | (lightmap data) | Remaining stride (unused by converter)|

**Texture name (32 bytes):** Null-terminated ASCII string following triangles.

#### Action Array (repeated NumActions times)

| Size | Type  | Field              | Description                           |
|------|-------|--------------------|---------------------------------------|
| 2    | int16 | NumAnimationKeys   | Keyframe count for this action        |
| 1    | uint8 | LockPositions      | If nonzero, position keys follow      |

If `LockPositions != 0`, followed by `NumAnimationKeys * 12` bytes of
`vec3_t` position keys (3 x float32 per key).

#### Bone Array (repeated NumBones times)

Each bone starts with a 1-byte `Dummy` flag:

- **Dummy != 0**: Null bone. No further data for this bone.
- **Dummy == 0**: Real bone, followed by:

| Size | Type     | Field    | Description                       |
|------|----------|----------|-----------------------------------|
| 32   | char[32] | Name     | Null-terminated ASCII bone name   |
| 2    | int16    | Parent   | Parent bone index (-1 = root)     |

Then for each action (NumActions times):
- `NumAnimationKeys * 12` bytes of position keys (3 x float32)
- `NumAnimationKeys * 12` bytes of rotation keys (3 x float32, Euler radians)

### 2.4 Bone Transform Pipeline

1. **Rotations** are stored in radians; convert to degrees: `angle_deg = rotation * (180 / PI)`
2. Apply `AngleMatrix()` (ZYX Euler convention) to build local 3x4 matrix.
3. Concatenate with parent world transform via `R_ConcatTransforms()`.
4. Vertices are transformed: `world_pos = VectorTransform(local_pos, bone_matrix) + bone_origin`
5. Normals are rotated (no translation): `world_normal = normalize(VectorRotate(local_normal, bone_matrix))`

### 2.5 UV Convention

BMD uses top-left origin; glTF uses bottom-left:

```
v_gltf = 1.0 - v_bmd
```

Reference: `BMD_SMD.cpp:755`

### 2.6 Non-Model BMD Stems

The following `.bmd` stems are data tables, not 3D models, and are skipped:

`item`, `minimap`, `itemsetting`, `petdata`, `gate`, `movereq`, `npcname`,
`quest`, `skill`, `filter`, `dialog`, `movelist`, `serverlist`, `chaosbox`,
`mixlist`

### 2.7 Output Format

Output: **GLB** (GLTF Binary 2.0)

- Single binary buffer containing POSITION (VEC3), NORMAL (VEC3), TEXCOORD_0 (VEC2), and indices (SCALAR) sections.
- Index type: uint16 when vertex count <= 65535, otherwise uint32.
- MU coordinates are swizzled from `(X, Y, Z-up)` to `(X, Y-up, Z)` and triangle winding is reversed after swizzle.
- Each source BMD mesh is emitted as one glTF primitive (shared vertex stream, per-primitive index accessor).
- Materials/images/textures are emitted from BMD mesh texture names (`*.jpg|*.tga|...` -> `*.png` URI beside the GLB).
- Generator tag: `mu-rust bmd_converter.py`

---

## 3. Textures

Source: `assets_convert.py`

### 3.1 Wrapper Formats

The MU client stores textures inside proprietary wrappers with fixed-size
headers prepended to standard image data.

| Extension | Header Size | Inner Format | Description                        |
|-----------|-------------|--------------|------------------------------------|
| `.ozj`    | 24 bytes    | JPEG         | Webzen JPEG wrapper                |
| `.ozj2`   | 24 bytes    | JPEG         | Same as OZJ                        |
| `.ozt`    | 4 bytes     | TGA          | Webzen TGA wrapper                 |
| `.ozb`    | 4 bytes     | BMP          | Webzen BMP wrapper                 |
| `.ozp`    | 4 bytes     | PNG          | Webzen PNG wrapper                 |

### 3.2 Direct Formats

| Extension       | Inner Format |
|-----------------|--------------|
| `.tga`          | TGA          |
| `.jpg` / `.jpeg`| JPEG         |
| `.bmp`          | BMP          |
| `.png`          | PNG          |

### 3.3 Deduplication Priority

When multiple source files map to the same output stem (e.g. `TileGrass01.OZJ`
and `TileGrass01.OZT`), the source with the lowest priority value wins:

| Priority | Extension(s)       |
|----------|--------------------|
| 0        | `.ozj`, `.ozj2`    |
| 1        | `.jpg`, `.jpeg`    |
| 2        | `.png`             |
| 3        | `.bmp`             |
| 4        | `.tga`             |
| 5        | `.ozt`             |
| 6        | `.ozb`             |
| 7        | `.ozp`             |

### 3.4 Conversion Pipeline

```
Source file
  -> Strip N header bytes (per format table)
  -> Validate non-empty, non-all-zero payload
  -> Pillow Image.open()
  -> Convert to RGBA mode
  -> Save as PNG
```

Output: **PNG** (RGBA), preserving the original directory layout.

---

## 4. Terrain Height Map

Source: `assets_convert.py` — `emit_terrain_height_json()`
Output: `terrain_height.json`

### 4.1 Source Format

Source file: `TerrainHeight.OZB` (inside each World directory)

The OZB wrapper has a 4-byte header; the inner payload is a standard BMP.

### 4.2 Height Extraction Modes

**8-bit mode** (`bits_per_pixel <= 8`):

- Each pixel is a raw byte value (0–255).
- Final height: `value * height_multiplier`
- Legacy fallback: raw bytes from BMP offset 1080 (for truncated files).

**24-bit mode** (`bits_per_pixel == 24`):

- BGR composite: `height = R + G * 256 + B * 65536`
- Final height: `composite + height_offset` (where `height_offset = -500.0`)
- Reference: `ZzzLodTerrain.cpp:734-746` (`OpenTerrainHeightNew`)

### 4.3 Height Multipliers

| World       | Multiplier | Notes                          |
|-------------|------------|--------------------------------|
| World55     | 3.0        | Login screen world             |
| All others  | 1.5        | Default                        |

For 24-bit BMPs the multiplier is forced to `1.0` (composite is already in
world units).

### 4.4 Output JSON Schema

```json
{
  "width": 256,
  "height": 256,
  "heights": [
    [0.0, 1.5, ...],   // Row 0: 256 float values
    ...                  // 256 rows total
  ],
  "metadata": {
    "source": "TerrainHeight.OZB",
    "source_bits_per_pixel": 8,
    "height_multiplier": 1.5,
    "height_offset": 0.0,
    "legacy_terrain_scale": 100.0,
    "source_unique_values": 142,
    "source_min": 0,
    "source_max": 255,
    "normalized_sample_min": 0.0,
    "normalized_sample_max": 255.0
  }
}
```

**Field descriptions:**

| Field                            | Type       | Description                                        |
|----------------------------------|------------|----------------------------------------------------|
| `width`                          | int        | Grid width in cells (always 256)                   |
| `height`                         | int        | Grid depth in cells (always 256)                   |
| `heights`                        | float[][]  | 256x256 array of raw sample values                 |
| `metadata.source`                | string     | Original filename                                  |
| `metadata.source_bits_per_pixel` | int        | 8 or 24 — extraction mode used                     |
| `metadata.height_multiplier`     | float      | Scale factor for Rust client (1.0, 1.5, or 3.0)   |
| `metadata.height_offset`         | float      | Additive offset (0.0 for 8-bit, -500.0 for 24-bit)|
| `metadata.legacy_terrain_scale`  | float      | Grid cell size in world units (100.0)              |
| `metadata.source_unique_values`  | int        | Count of distinct raw values (flatness indicator)  |
| `metadata.source_min`            | int        | Minimum raw sample value                           |
| `metadata.source_max`            | int        | Maximum raw sample value                           |
| `metadata.normalized_sample_min` | float      | Minimum value in `heights` array                   |
| `metadata.normalized_sample_max` | float      | Maximum value in `heights` array                   |

**Rust client formula:**
- 8-bit: `world_y = heights[row][col] * height_multiplier`
- 24-bit: `world_y = heights[row][col] + height_offset`

---

## 5. Terrain Tile Mapping

Source: `assets_convert.py` — `emit_terrain_map_json()`
Output: `<stem>.map.json`

### 5.1 Source Format

Source files: `EncTerrainN.map` or `TerrainN.map`

### 5.2 Encryption Detection

| Condition                              | Decryption                      |
|----------------------------------------|---------------------------------|
| Starts with `MAP\x01` (4 bytes)       | ModulusDecrypt (Season16+)      |
| Stem starts with `EncTerrain`          | MapFileDecrypt                  |
| Otherwise                              | Raw (unencrypted)               |

### 5.3 Decrypted Binary Layout

Total expected size: `2 + 65536 * 3 = 196610` bytes

| Offset  | Size   | Type    | Field      | Description                        |
|---------|--------|---------|------------|------------------------------------|
| 0       | 1      | uint8   | Version    | Map format version                 |
| 1       | 1      | uint8   | MapNumber  | World number                       |
| 2       | 65536  | uint8[] | Layer1     | Primary texture layer indices       |
| 65538   | 65536  | uint8[] | Layer2     | Secondary texture layer indices     |
| 131074  | 65536  | uint8[] | Alpha      | Blend alpha between layers (0-255) |

**Off-by-one note:** Unencrypted files may be 1 byte short (196609 instead of
196610) due to a bug in the original C++ `SaveTerrainMapping()`. The converter
pads with a zero byte.

### 5.4 Output JSON Schema

```json
{
  "header": {
    "version": 0,
    "map_number": 1
  },
  "terrain_size": 256,
  "layer_stats": {
    "layer1": { "min": 0, "max": 12, "mean": 3.14159, "unique_values": 8 },
    "layer2": { "min": 0, "max": 7, "mean": 1.23456, "unique_values": 5 },
    "alpha":  { "min": 0, "max": 255, "mean": 64.321, "unique_values": 128 }
  },
  "layer1": [[0, 1, ...], ...],
  "layer2": [[0, 0, ...], ...],
  "alpha":  [[0, 128, ...], ...]
}
```

**Field descriptions:**

| Field                        | Type     | Description                                        |
|------------------------------|----------|----------------------------------------------------|
| `header.version`             | int      | Map format version byte                            |
| `header.map_number`          | int      | World number                                       |
| `terrain_size`               | int      | Grid dimension (always 256)                        |
| `layer_stats.<name>.min`     | int      | Minimum value in layer                             |
| `layer_stats.<name>.max`     | int      | Maximum value in layer                             |
| `layer_stats.<name>.mean`    | float    | Mean value in layer                                |
| `layer_stats.<name>.unique_values` | int | Count of distinct values                          |
| `layer1`                     | int[][]  | 256x256 primary texture indices                    |
| `layer2`                     | int[][]  | 256x256 secondary texture indices                  |
| `alpha`                      | int[][]  | 256x256 alpha blend values (0-255)                 |

---

## 6. Terrain Attributes

Source: `assets_convert.py` — `emit_terrain_attribute_json()`
Output: `<stem>.json` (e.g. `EncTerrain1.json`)

### 6.1 Source Format

Source files: `EncTerrainN.att` or `TerrainN.att`

### 6.2 Encryption Detection

| Condition                              | Decryption                                 |
|----------------------------------------|--------------------------------------------|
| Starts with `ATT\x01` (4 bytes)       | ModulusDecrypt + Xor3Byte (Season16+)      |
| Stem starts with `EncTerrain`          | MapFileDecrypt + Xor3Byte                  |
| Otherwise                              | Raw (unencrypted)                          |

### 6.3 Decrypted Binary Layout

**Standard format (8-bit):** Total = 65540 bytes

| Offset | Size  | Type    | Field      | Description                        |
|--------|-------|---------|------------|------------------------------------|
| 0      | 1     | uint8   | Version    | Attribute format version           |
| 1      | 1     | uint8   | MapNumber  | World number                       |
| 2      | 1     | uint8   | Width      | Grid width (typically 0 = 256)     |
| 3      | 1     | uint8   | Height     | Grid height (typically 0 = 256)    |
| 4      | 65536 | uint8[] | TileFlags  | One byte per tile (8-bit flags)    |

**Extended format (16-bit):** Total = 131076 bytes

| Offset | Size   | Type     | Field      | Description                       |
|--------|--------|----------|------------|-----------------------------------|
| 0      | 1      | uint8    | Version    | Attribute format version          |
| 1      | 1      | uint8    | MapNumber  | World number                      |
| 2      | 1      | uint8    | Width      | Grid width                        |
| 3      | 1      | uint8    | Height     | Grid height                       |
| 4      | 131072 | uint16[] | TileFlags  | Two bytes (LE) per tile           |

**Off-by-one note:** Unencrypted `.att` files may be 1 byte short (65539 or
131075) due to a bug in `SaveTerrainAttribute()`. The converter pads with zero.

### 6.4 Tile Flag Definitions

Source: `_define.h`

| Flag Name        | Value    | Hex      | Description                       |
|------------------|----------|----------|-----------------------------------|
| TW_SAFEZONE      | 1        | `0x0001` | Safe zone (no PvP)               |
| TW_CHARACTER     | 2        | `0x0002` | Character present                 |
| TW_NOMOVE        | 4        | `0x0004` | Movement blocked                  |
| TW_NOGROUND      | 8        | `0x0008` | No ground (void/hole)            |
| TW_WATER         | 16       | `0x0010` | Water surface                     |
| TW_ACTION        | 32       | `0x0020` | Action trigger zone               |
| TW_HEIGHT        | 64       | `0x0040` | Height modifier active            |
| TW_CAMERA_UP     | 128      | `0x0080` | Force camera elevation            |
| TW_NOATTACKZONE  | 256      | `0x0100` | Attack-free zone (16-bit only)   |
| TW_ATT1          | 512      | `0x0200` | Custom attribute 1 (16-bit only) |
| TW_ATT2          | 1024     | `0x0400` | Custom attribute 2 (16-bit only) |
| TW_ATT3          | 2048     | `0x0800` | Custom attribute 3 (16-bit only) |
| TW_ATT4          | 4096     | `0x1000` | Custom attribute 4 (16-bit only) |
| TW_ATT5          | 8192     | `0x2000` | Custom attribute 5 (16-bit only) |
| TW_ATT6          | 16384    | `0x4000` | Custom attribute 6 (16-bit only) |
| TW_ATT7          | 32768    | `0x8000` | Custom attribute 7 (16-bit only) |

Flags `TW_NOATTACKZONE` through `TW_ATT7` (bits 8-15) are only available in
the extended 16-bit format.

### 6.5 Output JSON Schema

```json
{
  "header": {
    "version": 0,
    "map_number": 1,
    "width": 0,
    "height": 0
  },
  "is_extended": false,
  "terrain_size": 256,
  "terrain_data": [
    [1, 0, 4, ...],   // Row 0: 256 flag values
    ...                 // 256 rows total
  ]
}
```

**Field descriptions:**

| Field                  | Type     | Description                                         |
|------------------------|----------|-----------------------------------------------------|
| `header.version`       | int      | Attribute format version byte                       |
| `header.map_number`    | int      | World number                                        |
| `header.width`         | int      | Raw width byte from header (0 = 256)                |
| `header.height`        | int      | Raw height byte from header (0 = 256)               |
| `is_extended`          | bool     | `true` if 16-bit flags, `false` if 8-bit            |
| `terrain_size`         | int      | Grid dimension (always 256)                         |
| `terrain_data`         | int[][]  | 256x256 array of per-tile flag bitmasks             |

---

## 7. Scene Objects

Source: `assets_convert.py` — `emit_scene_objects_json()`
Output: `scene_objects.json`

### 7.1 Source Format

Source files: `EncTerrainN.obj`

### 7.2 Entry Sizes by Version

| Version | Entry Size (bytes) |
|---------|--------------------|
| 0       | 30                 |
| 1       | 32                 |
| 2       | 33                 |
| 3       | 45                 |
| 4       | 46                 |
| 5       | 54                 |

### 7.3 Base Entry Structure (30 bytes)

All versions share a common prefix:

| Offset | Size | Type    | Field    | Description                            |
|--------|------|---------|----------|----------------------------------------|
| 0      | 2    | int16   | Type     | Object type ID (model_index = Type + 1)|
| 2      | 4    | float32 | Pos.X    | World position X                       |
| 6      | 4    | float32 | Pos.Y    | World position Y (MU coordinate)       |
| 10     | 4    | float32 | Pos.Z    | World position Z (MU coordinate)       |
| 14     | 4    | float32 | Angle.X  | Rotation around X axis (degrees)       |
| 18     | 4    | float32 | Angle.Y  | Rotation around Y axis (MU coordinate) |
| 22     | 4    | float32 | Angle.Z  | Rotation around Z axis (MU coordinate) |
| 26     | 4    | float32 | Scale    | Uniform scale factor                   |

### 7.4 Coordinate Swizzle

MU Online uses a different coordinate convention than Bevy (Y-up):

```
Bevy.X = MU.X       (position[0])
Bevy.Y = MU.Z       (position[1] in JSON)
Bevy.Z = MU.Y       (position[2] in JSON)
```

`position` is emitted in Bevy coordinates. `rotation` is emitted in MU angle
order (`Angle.X`, `Angle.Y`, `Angle.Z`, degrees), and the client applies MU's
`AngleMatrix` convention (`(Z * Y) * X`) plus MU→Bevy basis conversion at
runtime.

Legacy converted assets may contain swizzled Euler rotations. Runtime behavior
for both formats is selected by `metadata.rotation_encoding`.

### 7.5 Decryption

The converter tries multiple decryption candidates and scores them by data
sanity (valid ranges for position, angle, scale, type):

1. `map_file_decrypt` (MapFileDecrypt)
2. Raw (no decryption)
3. `apply_bux_convert(map_file_decrypt(...))` (MapFileDecrypt + Xor3Byte)
4. `apply_bux_convert(raw)` (Xor3Byte only)

The candidate with the highest sanity score (>= 0.45) is selected.

### 7.6 Output JSON Schema

```json
{
  "objects": [
    {
      "id": "obj_00000",
      "type": 148,
      "model": "data/Object1/Object149.glb",
      "position": [1234.5, 170.0, 5678.9],
      "rotation": [0.0, 45.0, 0.0],
      "scale": [1.0, 1.0, 1.0],
      "properties": {
        "model_renderable": true
      }
    }
  ],
  "metadata": {
    "source": "EncTerrain1.obj",
    "version": 0,
    "map_number": 1,
    "object_count": 500,
    "entry_size": 30,
    "decode_name": "map_file_decrypt",
    "layout_name": "version_map_count",
    "decode_score": 0.9500,
    "rotation_encoding": "mu_angles_degrees",
    "rotation_convention": "mu_anglematrix_zyx_degrees"
  }
}
```

**`objects[]` field descriptions:**

| Field        | Type       | Description                                         |
|--------------|------------|-----------------------------------------------------|
| `id`         | string     | Unique ID: `obj_NNNNN` (zero-padded index)          |
| `type`       | int        | Object type from binary (model_index = type + 1)    |
| `model`      | string     | Resolved path to GLB model file                     |
| `position`   | float[3]   | `[X, Y, Z]` in Bevy coordinates (Y-up, swizzled)   |
| `rotation`   | float[3]   | MU angles `[X, Y, Z]` in degrees (converted at runtime) |
| `scale`      | float[3]   | `[X, Y, Z]` uniform scale (all three are equal)     |
| `properties` | object     | Optional properties (see below)                     |

**`properties` sub-fields:**

| Field                       | Type   | Description                                  |
|-----------------------------|--------|----------------------------------------------|
| `model_renderable`          | bool   | Whether the referenced GLB is valid          |
| `model_validation_reason`   | string | (optional) Reason if model is not renderable |

**`metadata` field descriptions:**

| Field          | Type   | Description                                        |
|----------------|--------|----------------------------------------------------|
| `source`       | string | Original filename                                  |
| `version`      | int    | Format version from header                         |
| `map_number`   | int    | World number from header                           |
| `object_count` | int    | Total number of objects decoded                    |
| `entry_size`   | int    | Bytes per object entry                             |
| `decode_name`  | string | Decryption method used                             |
| `layout_name`  | string | Header layout interpretation used                  |
| `decode_score` | float  | Sanity score of the selected decoding (0.0–1.0+)  |
| `rotation_encoding` | string | Rotation payload encoding (`mu_angles_degrees` or legacy swizzled formats) |
| `rotation_convention` | string | Rotation semantics used by runtime (`mu_anglematrix_zyx_degrees`) |

---

## 8. Camera Tour

Source: `assets_convert.py` — `emit_camera_tour_json()`
Output: `camera_tour.json` (normalized) + `CWScriptN.cws.json` (raw)

### 8.1 Source Format

Source files: `CWScriptN.cws`

### 8.2 Binary Layout

| Offset | Size | Type   | Field          | Description                     |
|--------|------|--------|----------------|---------------------------------|
| 0      | 4    | uint32 | Magic          | `0x00535743` (ASCII `CWS\0`)   |
| 4      | 4/8  | int    | WaypointCount  | Number of waypoints (4 or 8 byte width, auto-detected) |

Followed by `WaypointCount` waypoint structs.

**Waypoint struct (28 bytes):**

| Offset | Size | Type    | Field         | Description                         |
|--------|------|---------|---------------|-------------------------------------|
| 0      | 4    | int32   | Index         | Grid cell index (row * 256 + col)   |
| 4      | 4    | float32 | Camera.X      | Camera position X                   |
| 8      | 4    | float32 | Camera.Y      | Camera position Y (MU convention)   |
| 12     | 4    | float32 | Camera.Z      | Camera position Z (MU convention)   |
| 16     | 4    | int32   | DelayFrames   | Delay at this waypoint (in frames)  |
| 20     | 4    | float32 | MoveAccel     | Movement acceleration               |
| 24     | 4    | float32 | DistanceLevel  | Camera distance level               |

### 8.3 Raw Output JSON Schema

```json
{
  "magic": 5525315,
  "waypoint_count": 8,
  "waypoints": [
    {
      "index": 32896,
      "camera_x": 12800.0,
      "camera_y": 12800.0,
      "camera_z": 170.0,
      "delay": 0,
      "move_accel": 16.0,
      "distance_level": 8.0,
      "grid_x": 128,
      "grid_y": 128
    }
  ],
  "grid_bounds": {
    "x": { "min": 100, "max": 200 },
    "y": { "min": 100, "max": 200 }
  }
}
```

| Field                    | Type   | Description                                 |
|--------------------------|--------|---------------------------------------------|
| `magic`                  | int    | File magic (always `0x00535743` = 5525315)  |
| `waypoint_count`         | int    | Number of waypoints                         |
| `waypoints[].index`      | int    | Flat grid index                             |
| `waypoints[].camera_x`   | float  | Camera X position                           |
| `waypoints[].camera_y`   | float  | Camera Y (MU = Bevy Z)                     |
| `waypoints[].camera_z`   | float  | Camera Z (MU = Bevy Y)                     |
| `waypoints[].delay`      | int    | Delay in frames                             |
| `waypoints[].move_accel` | float  | Movement acceleration                       |
| `waypoints[].distance_level` | float | Camera distance level                   |
| `waypoints[].grid_x`     | int    | `index % 256`                              |
| `waypoints[].grid_y`     | int    | `index // 256`                             |
| `grid_bounds`            | object | Bounding box of grid coordinates            |

### 8.4 Normalized Output JSON Schema

```json
{
  "waypoints": [
    {
      "index": 32896,
      "position": [12800.0, 490.0, 12800.0],
      "look_at": [13150.0, 170.0, 12800.0],
      "move_acceleration": 16.0,
      "distance_level": 8.0,
      "delay": 0.0
    }
  ],
  "loop": true,
  "blend_distance": 300.0,
  "interpolation": "smooth"
}
```

| Field                               | Type     | Description                                      |
|-------------------------------------|----------|--------------------------------------------------|
| `waypoints[].index`                 | int      | Original grid cell index                         |
| `waypoints[].position`              | float[3] | Camera position `[X, Y, Z]` (Bevy, Y-up)        |
| `waypoints[].look_at`               | float[3] | Look-at target `[X, Y, Z]` (Bevy, Y-up)         |
| `waypoints[].move_acceleration`     | float    | Movement acceleration (clamped >= 0.1)           |
| `waypoints[].distance_level`        | float    | Camera distance (clamped >= 5.0)                 |
| `waypoints[].delay`                 | float    | Delay in seconds (frames / 60)                   |
| `loop`                              | bool     | Whether to loop the tour                         |
| `blend_distance`                    | float    | Blending distance between waypoints (world units)|
| `interpolation`                     | string   | Interpolation mode (`"smooth"`)                  |

### 8.5 Coordinate Transform

The normalized output converts MU coordinates to Bevy (Y-up):

```
Bevy.X = MU.Camera.X
Bevy.Y = MU.Camera.Z + elevation
Bevy.Z = MU.Camera.Y
```

Where `elevation = max(140.0, distance_level * 40.0)` and
`look_ahead = max(350.0, distance_level * 80.0)`.

---

## 9. Terrain Config

Source: `assets_convert.py` — `emit_default_terrain_config()`
Output: `terrain_config.json`

This is a **generated placeholder** created when no terrain configuration
exists in the legacy data. It provides sensible defaults for the Bevy client.

### 9.1 Output JSON Schema

```json
{
  "size": {
    "width": 256,
    "depth": 256,
    "scale": 100.0
  },
  "height_multiplier": 1.5,
  "legacy_terrain_scale": 100.0,
  "texture_layers": [
    { "id": "grass01",  "path": "data/World1/TileGrass01.png",  "scale": 1.0 },
    { "id": "ground01", "path": "data/World1/TileGround01.png", "scale": 1.0 },
    { "id": "rock01",   "path": "data/World1/TileRock01.png",   "scale": 1.0 }
  ],
  "alpha_map": "data/World1/AlphaTile01.png",
  "lightmap": "data/World1/TerrainLight.png",
  "metadata": {
    "generated_placeholder": true,
    "reason": "terrain_config.json missing in legacy data",
    "world": 1
  }
}
```

**Field descriptions:**

| Field                            | Type     | Description                                      |
|----------------------------------|----------|--------------------------------------------------|
| `size.width`                     | int      | Grid width in cells (256)                        |
| `size.depth`                     | int      | Grid depth in cells (256)                        |
| `size.scale`                     | float    | World units per grid cell (100.0)                |
| `height_multiplier`              | float    | Height sample multiplier (1.5)                   |
| `legacy_terrain_scale`           | float    | Original client terrain scale (100.0)            |
| `texture_layers[].id`            | string   | Layer identifier                                 |
| `texture_layers[].path`          | string   | Path to texture PNG relative to asset root       |
| `texture_layers[].scale`         | float    | Texture tiling scale                             |
| `alpha_map`                      | string   | Path to alpha blend texture                      |
| `lightmap`                       | string   | Path to lightmap texture                         |
| `metadata.generated_placeholder` | bool     | Always `true` — indicates this is auto-generated |
| `metadata.reason`                | string   | Why a placeholder was created                    |
| `metadata.world`                 | int      | World number                                     |

---

## 10. Encryption Algorithms

### 10.1 MapFileDecrypt

Source: `MuCrypto.cpp:65-77`, `assets_convert.py`

XOR-based stream cipher with a rolling key:

```
XOR_KEY = [0xD1, 0x73, 0x52, 0xF6, 0xD2, 0x9A, 0xCB, 0x27,
           0x3E, 0xAF, 0x59, 0x31, 0x37, 0xB3, 0xE7, 0xA2]
KEY_SEED = 0x5E

rolling_key = KEY_SEED
for i in range(len(data)):
    output[i] = ((data[i] ^ XOR_KEY[i % 16]) - rolling_key) & 0xFF
    rolling_key = (data[i] + 0x3D) & 0xFF
```

Used by: BMD version 0x0C, encrypted `.map` files, `.obj` files.

### 10.2 Xor3Byte

Source: `mu_terrain_decrypt.cpp`, `assets_convert.py`

Simple 3-byte repeating XOR:

```
KEY = [0xFC, 0xCF, 0xAB]

for i in range(len(data)):
    output[i] = data[i] ^ KEY[i % 3]
```

Used by: `.att` files (applied after MapFileDecrypt or ModulusDecrypt).

### 10.3 ModulusDecrypt

Source: `MuCrypto.cpp:220-262`, `mu_terrain_decrypt.cpp`

Multi-cipher block decryption using 8 Crypto++ algorithms, selected by two
algorithm bytes embedded in the encrypted payload.

**Key:** `"webzen#@!01webzen#@!01webzen#@!0"` (32 bytes)

**Cipher selection** (`algorithm & 7`):

| Index | Cipher   | Library       |
|-------|----------|---------------|
| 0     | TEA      | Crypto++      |
| 1     | ThreeWay | Crypto++      |
| 2     | CAST-128 | Crypto++      |
| 3     | RC5      | Crypto++      |
| 4     | RC6      | Crypto++      |
| 5     | MARS     | Crypto++      |
| 6     | IDEA     | Crypto++      |
| 7     | GOST     | Crypto++      |

**Critical:** Keys are set using `T::DEFAULT_KEYLENGTH`, not the full 32-byte
key length, because each cipher has a different default key size.

**Decryption flow:**

1. Read `algorithm_1 = buf[1]`, `algorithm_2 = buf[0]`.
2. Instantiate cipher1 from `algorithm_1`. Block-align to 1024 bytes.
3. Decrypt up to 3 regions of the body using cipher1.
4. Extract `key_2` (32 bytes) from decrypted data at offset 2.
5. Instantiate cipher2 from `algorithm_2` using `key_2`.
6. Decrypt remaining body at offset 34 using cipher2.
7. Strip the 34-byte header.

**File-type post-processing:**

- **ATT:** `Strip 4-byte magic` -> `ModulusDecrypt` -> `Xor3Byte`
- **MAP:** `Strip 4-byte magic` -> `ModulusDecrypt` (no Xor3Byte)
- **BMD (0x0E):** Body is wrapped in fake `MAP\x01` header before decryption.

### 10.4 LEA-256 ECB

Source: `bmd_converter.py`

LEA (Lightweight Encryption Algorithm) — Korean block cipher standardized by
KISA. Used for Season20 BMD files (version `0x0F`).

**Parameters:**

| Property   | Value                                                                          |
|------------|--------------------------------------------------------------------------------|
| Block size | 16 bytes                                                                       |
| Key size   | 32 bytes (256-bit)                                                             |
| Rounds     | 32                                                                             |
| Mode       | ECB (each 16-byte block decrypted independently)                               |
| Key (hex)  | `cc 50 45 13 c2 a6 57 4e d6 9a 45 89 bf 2f bc d9 39 b3 b3 bd 50 bd cc b6 85 46 d1 d6 16 54 e0 87` |

**Constraint:** Encrypted payload size must be 16-byte aligned.

---

## 11. Terrain Constants

| Constant                         | Value   | Description                                    |
|----------------------------------|---------|------------------------------------------------|
| `TERRAIN_SIZE`                   | 256     | Grid dimension (256 x 256 tiles)               |
| `TERRAIN_TILE_COUNT`             | 65536   | Total tiles per layer (256 * 256)              |
| `LEGACY_TERRAIN_SCALE`           | 100.0   | World units per grid cell                      |
| `DEFAULT_WORLD_HEIGHT_MULTIPLIER`| 1.5     | Height multiplier for most worlds              |
| `LOGIN_WORLD_HEIGHT_MULTIPLIER`  | 3.0     | Height multiplier for World55 (login screen)   |
| `LEGACY_OZB_TERRAIN_HEADER`     | 1080    | BMP header offset for legacy 8-bit extraction  |
| `g_fMinHeight`                   | -500.0  | Height offset for 24-bit terrain (from C++)    |

---

## 12. Extension Catalog

### 12.1 Converted Extensions

| Extension    | Category       | Output Format          | Encryption                   |
|--------------|----------------|------------------------|------------------------------|
| `.bmd`       | 3D Model       | `.glb`                 | None / MapFile / Modulus / LEA |
| `.ozj`       | Texture        | `.png`                 | None (24B header)            |
| `.ozj2`      | Texture        | `.png`                 | None (24B header)            |
| `.ozt`       | Texture        | `.png`                 | None (4B header)             |
| `.ozb`       | Texture/Height | `.png` / `.json`       | None (4B header)             |
| `.ozp`       | Texture        | `.png`                 | None (4B header)             |
| `.tga`       | Texture        | `.png`                 | None                         |
| `.bmp`       | Texture        | `.png`                 | None                         |
| `.jpg`       | Texture        | `.png`                 | None                         |
| `.jpeg`      | Texture        | `.png`                 | None                         |
| `.png`       | Texture        | `.png` (pass-through)  | None                         |
| `.map`       | Terrain Tiles  | `.map.json`            | None / MapFile / Modulus     |
| `.att`       | Terrain Attrs  | `.json`                | None / MapFile+Xor / Modulus+Xor |
| `.obj`       | Scene Objects  | `scene_objects.json`   | MapFile (Season20: unknown)  |
| `.cws`       | Camera Tour    | `camera_tour.json`     | None                         |

### 12.2 Ignored Extensions

These extensions are present in the client data but are not processed by the
conversion pipeline:

| Extension | Description                                            |
|-----------|--------------------------------------------------------|
| `.ozd`    | Encrypted data archive                                 |
| `.ozg`    | Encrypted graphics archive                             |
| `.psd`    | Photoshop source (development artifact)                |
| `.smd`    | Studiomdl Data (intermediate format, not used)         |
| `.fbx`    | Autodesk FBX (not present in release builds)           |
| `.txt`    | Text configuration / data tables                       |
| `.csv`    | Comma-separated data tables                            |
| `.xml`    | XML configuration files                                |
| `.lua`    | Lua scripts (UI / event)                               |
| `.wav`    | Sound effects                                          |
| `.mp3`    | Music                                                  |
| `.ogg`    | Music / sound                                          |
| `.dll`    | Client plugins / libraries                             |
| `.exe`    | Client executables                                     |
| `.dat`    | Binary data tables                                     |
