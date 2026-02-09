#!/usr/bin/env python3
"""
Remaster GLB textures with a text-to-image API (Nano Banana 3 Flash workflow).

Pipeline:
1. Recursively scan an input directory for `.glb` files.
2. Load each GLB in memory, extract embedded/external image bytes.
3. Select textures (all images by default, optional baseColor-only mode).
4. Skip textures with transparency.
5. Save original texture bytes to `assets/remaster_raw/{same_glb_relative_path}/...`.
6. Perform one API request per eligible texture.
7. Convert API output to JPEG and inject into the GLB.
8. Save remastered GLBs to `assets/remaster/{same_relative_glb_path}`.
"""

from __future__ import annotations

import argparse
import base64
import json
import logging
import os
import random
import re
import struct
import sys
import time
from dataclasses import dataclass
from io import BytesIO
from pathlib import Path
from typing import Any, Dict, Iterable, List, Optional, Set, Tuple
from urllib.parse import unquote

try:
    import httpx
except ImportError:  # pragma: no cover - runtime dependency check
    httpx = None

try:
    from PIL import Image, UnidentifiedImageError
except ImportError:  # pragma: no cover - runtime dependency check
    Image = None
    UnidentifiedImageError = Exception

JSON_CHUNK_TYPE = 0x4E4F534A
BIN_CHUNK_TYPE = 0x004E4942
GLTF_MAGIC = 0x46546C67

MIME_BY_EXT = {
    ".png": "image/png",
    ".jpg": "image/jpeg",
    ".jpeg": "image/jpeg",
    ".webp": "image/webp",
    ".bmp": "image/bmp",
    ".gif": "image/gif",
    ".tga": "image/x-tga",
}

EXT_BY_MIME = {
    "image/png": ".png",
    "image/jpeg": ".jpg",
    "image/jpg": ".jpg",
    "image/webp": ".webp",
    "image/bmp": ".bmp",
    "image/gif": ".gif",
    "image/x-tga": ".tga",
}

SAFE_NAME_RE = re.compile(r"[^a-zA-Z0-9._-]+")

DEFAULT_PROMPT = (
    "Remaster this game texture in high quality, preserving composition, UV layout, "
    "and semantic details. Keep it tile-friendly, artifact-free, and production-ready."
)

DEFAULT_GEMINI_ENDPOINT = "https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent"
DEFAULT_OPENAI_ENDPOINT = "https://api.openai.com/v1/responses"


@dataclass
class TextureSource:
    image_index: int
    source_kind: str  # bufferview | data_uri | external_uri
    raw_bytes: bytes
    mime_type: Optional[str]
    logical_name: str


@dataclass
class TexturePrepared:
    has_transparency: bool
    upload_png_bytes: bytes
    width: int
    height: int


@dataclass
class FileReport:
    rel_glb: Path
    images_total: int = 0
    targeted_images: int = 0
    attempted_requests: int = 0
    remastered: int = 0
    skipped_transparent: int = 0
    skipped_not_target: int = 0
    extraction_failed: int = 0
    api_failed: int = 0


@dataclass
class RunReport:
    files_total: int = 0
    files_ok: int = 0
    files_failed: int = 0
    images_total: int = 0
    targeted_images: int = 0
    attempted_requests: int = 0
    remastered: int = 0
    skipped_transparent: int = 0
    skipped_not_target: int = 0
    extraction_failed: int = 0
    api_failed: int = 0


def parse_args(argv: Iterable[str]) -> argparse.Namespace:
    parser = argparse.ArgumentParser(
        description="Remaster textures from GLB files and write new GLBs preserving relative paths.",
    )
    parser.add_argument(
        "input_dir",
        type=Path,
        help="Directory containing GLB assets (searched recursively).",
    )
    parser.add_argument(
        "--raw-dir",
        type=Path,
        default=Path("assets/remaster_raw"),
        help="Directory for original texture backups (default: assets/remaster_raw).",
    )
    parser.add_argument(
        "--out-dir",
        type=Path,
        default=Path("assets/remaster"),
        help="Directory for remastered GLBs (default: assets/remaster).",
    )
    parser.add_argument(
        "--api-key",
        default=os.getenv("NANO_BANANA_API_KEY") or os.getenv("GEMINI_API_KEY"),
        help="API key for Nano Banana/Gemini endpoint. Defaults from env NANO_BANANA_API_KEY or GEMINI_API_KEY.",
    )
    parser.add_argument(
        "--model",
        default="nano-banana-3-flash",
        help="Model name (default: nano-banana-3-flash).",
    )
    parser.add_argument(
        "--endpoint-template",
        default=DEFAULT_GEMINI_ENDPOINT,
        help=(
            "HTTP endpoint template. Supports {model}. "
            "Default is Gemini-style generateContent endpoint."
        ),
    )
    parser.add_argument(
        "--api-mode",
        choices=["gemini", "openai"],
        default="gemini",
        help="Request format for the image remaster API (default: gemini).",
    )
    parser.add_argument(
        "--prompt",
        default=DEFAULT_PROMPT,
        help="Prompt sent with each texture image.",
    )
    parser.add_argument(
        "--jpeg-quality",
        type=int,
        default=90,
        help="JPEG quality used for remastered textures (1-100, default: 90).",
    )
    parser.add_argument(
        "--max-retries",
        type=int,
        default=3,
        help="Max retry attempts for API calls on transient errors (default: 3).",
    )
    parser.add_argument(
        "--timeout-seconds",
        type=float,
        default=120.0,
        help="API request timeout in seconds (default: 120).",
    )
    parser.add_argument(
        "--only-base-color",
        action="store_true",
        help="Restrict remastering to baseColor textures only (default: process all textures).",
    )
    parser.add_argument(
        "--dry-run",
        action="store_true",
        help="Inspect files and show intended actions without writing outputs or calling API.",
    )
    parser.add_argument(
        "--verbose",
        action="store_true",
        help="Enable debug logging.",
    )
    args = parser.parse_args(list(argv))

    if args.jpeg_quality < 1 or args.jpeg_quality > 100:
        parser.error("--jpeg-quality must be between 1 and 100")
    if args.max_retries < 0:
        parser.error("--max-retries must be >= 0")
    if args.timeout_seconds <= 0:
        parser.error("--timeout-seconds must be > 0")

    return args


def configure_logging(verbose: bool) -> None:
    logging.basicConfig(
        level=logging.DEBUG if verbose else logging.INFO,
        format="%(levelname)s: %(message)s",
    )
    logging.getLogger("PIL").setLevel(logging.INFO)


def align4(value: int) -> int:
    return (value + 3) & ~3


def safe_texture_name(name: str) -> str:
    cleaned = SAFE_NAME_RE.sub("_", name).strip("._-")
    return cleaned or "texture"


def guess_mime_from_name(name: str) -> Optional[str]:
    suffix = Path(name).suffix.lower()
    return MIME_BY_EXT.get(suffix)


def extension_from_mime(mime: Optional[str]) -> str:
    if not mime:
        return ".bin"
    return EXT_BY_MIME.get(mime.lower(), ".bin")


def load_glb_payload(path: Path) -> Tuple[Dict[str, Any], bytes]:
    data = path.read_bytes()
    if len(data) < 20:
        raise ValueError("GLB too small")

    magic, version, total_length = struct.unpack_from("<III", data, 0)
    if magic != GLTF_MAGIC:
        raise ValueError("Invalid GLB magic")
    if version != 2:
        raise ValueError(f"Unsupported GLB version: {version}")
    if total_length > len(data):
        raise ValueError("GLB is truncated")

    offset = 12
    json_chunk: Optional[bytes] = None
    bin_chunk: bytes = b""

    while offset + 8 <= len(data):
        chunk_len, chunk_type = struct.unpack_from("<II", data, offset)
        offset += 8
        chunk_end = offset + chunk_len
        if chunk_end > len(data):
            raise ValueError("GLB chunk exceeds file size")

        chunk_data = data[offset:chunk_end]
        offset = chunk_end

        if chunk_type == JSON_CHUNK_TYPE and json_chunk is None:
            json_chunk = chunk_data
        elif chunk_type == BIN_CHUNK_TYPE and not bin_chunk:
            bin_chunk = chunk_data

    if json_chunk is None:
        raise ValueError("GLB missing JSON chunk")

    payload = json.loads(json_chunk.decode("utf-8").rstrip(" \t\r\n\x00"))
    if not isinstance(payload, dict):
        raise ValueError("GLB JSON root is not an object")

    return payload, bin_chunk


def build_glb(payload: Dict[str, Any], binary_blob: bytes) -> bytes:
    json_bytes = json.dumps(payload, separators=(",", ":"), ensure_ascii=False).encode("utf-8")
    json_pad = align4(len(json_bytes)) - len(json_bytes)
    if json_pad:
        json_bytes += b" " * json_pad

    bin_pad = align4(len(binary_blob)) - len(binary_blob)
    if bin_pad:
        binary_blob += b"\x00" * bin_pad

    total_length = 12 + 8 + len(json_bytes) + 8 + len(binary_blob)

    out = bytearray()
    out += struct.pack("<III", GLTF_MAGIC, 2, total_length)
    out += struct.pack("<II", len(json_bytes), JSON_CHUNK_TYPE)
    out += json_bytes
    out += struct.pack("<II", len(binary_blob), BIN_CHUNK_TYPE)
    out += binary_blob
    return bytes(out)


def decode_data_uri(uri: str) -> Tuple[bytes, Optional[str]]:
    # Format: data:[<mime>][;base64],<data>
    if not uri.startswith("data:"):
        raise ValueError("Not a data URI")

    comma = uri.find(",")
    if comma == -1:
        raise ValueError("Invalid data URI")

    header = uri[5:comma]
    payload = uri[comma + 1 :]

    is_base64 = False
    mime: Optional[str] = None

    if header:
        parts = header.split(";")
        if parts and "/" in parts[0]:
            mime = parts[0]
        if any(part.lower() == "base64" for part in parts[1:] if part):
            is_base64 = True

    if is_base64:
        data = base64.b64decode(payload)
    else:
        data = unquote(payload).encode("utf-8")

    return data, mime


def extract_image_source(
    image_index: int,
    image_obj: Dict[str, Any],
    payload: Dict[str, Any],
    binary_blob: bytes,
    glb_path: Path,
) -> TextureSource:
    logical_name = image_obj.get("name") or f"image_{image_index:03d}"
    mime_type = image_obj.get("mimeType")

    if isinstance(image_obj.get("bufferView"), int):
        buffer_views = payload.get("bufferViews")
        if not isinstance(buffer_views, list):
            raise ValueError("GLB has image.bufferView but no bufferViews list")

        view_index = image_obj["bufferView"]
        if view_index < 0 or view_index >= len(buffer_views):
            raise ValueError(f"image[{image_index}] references invalid bufferView {view_index}")

        view = buffer_views[view_index]
        if not isinstance(view, dict):
            raise ValueError(f"bufferView[{view_index}] is not an object")

        byte_offset = int(view.get("byteOffset", 0))
        byte_length = int(view.get("byteLength", 0))
        if byte_offset < 0 or byte_length <= 0:
            raise ValueError(f"image[{image_index}] has invalid byte range")
        end = byte_offset + byte_length
        if end > len(binary_blob):
            raise ValueError(f"image[{image_index}] bufferView exceeds BIN chunk")

        raw = binary_blob[byte_offset:end]
        return TextureSource(
            image_index=image_index,
            source_kind="bufferview",
            raw_bytes=raw,
            mime_type=mime_type,
            logical_name=str(logical_name),
        )

    uri = image_obj.get("uri")
    if not isinstance(uri, str) or not uri:
        raise ValueError(f"image[{image_index}] has neither bufferView nor uri")

    if uri.startswith("data:"):
        raw, uri_mime = decode_data_uri(uri)
        return TextureSource(
            image_index=image_index,
            source_kind="data_uri",
            raw_bytes=raw,
            mime_type=mime_type or uri_mime,
            logical_name=str(logical_name),
        )

    rel_uri = unquote(uri.replace("\\", "/"))
    texture_path = (glb_path.parent / rel_uri).resolve()
    if not texture_path.exists() or not texture_path.is_file():
        raise ValueError(f"External texture not found: {uri}")

    raw = texture_path.read_bytes()
    return TextureSource(
        image_index=image_index,
        source_kind="external_uri",
        raw_bytes=raw,
        mime_type=mime_type or guess_mime_from_name(rel_uri),
        logical_name=Path(rel_uri).name or str(logical_name),
    )


def prepare_texture(raw_bytes: bytes) -> TexturePrepared:
    if Image is None:
        raise RuntimeError("Pillow is required. Install with `pip install Pillow`.")

    try:
        with Image.open(BytesIO(raw_bytes)) as img:
            rgba = img.convert("RGBA")
            alpha = rgba.getchannel("A")
            min_alpha, _max_alpha = alpha.getextrema()
            has_transparency = min_alpha < 255

            rgb = img.convert("RGB")
            out = BytesIO()
            rgb.save(out, format="PNG")
            upload_png = out.getvalue()

            width, height = img.size

        return TexturePrepared(
            has_transparency=has_transparency,
            upload_png_bytes=upload_png,
            width=width,
            height=height,
        )
    except UnidentifiedImageError as exc:
        raise ValueError(f"Unsupported/unknown texture format: {exc}") from exc


def encode_jpeg(image_bytes: bytes, quality: int) -> bytes:
    if Image is None:
        raise RuntimeError("Pillow is required. Install with `pip install Pillow`.")

    with Image.open(BytesIO(image_bytes)) as img:
        rgb = img.convert("RGB")
        out = BytesIO()
        rgb.save(out, format="JPEG", quality=quality, optimize=True)
        return out.getvalue()


def extract_image_bytes_from_response(payload: Dict[str, Any]) -> Tuple[bytes, Optional[str]]:
    # Gemini-style response: candidates[*].content.parts[*].inlineData{mimeType,data}
    candidates = payload.get("candidates")
    if isinstance(candidates, list):
        for candidate in candidates:
            if not isinstance(candidate, dict):
                continue
            content = candidate.get("content")
            if not isinstance(content, dict):
                continue
            parts = content.get("parts")
            if not isinstance(parts, list):
                continue
            for part in parts:
                if not isinstance(part, dict):
                    continue
                inline_data = part.get("inlineData") or part.get("inline_data")
                if isinstance(inline_data, dict):
                    data_b64 = inline_data.get("data")
                    mime = inline_data.get("mimeType") or inline_data.get("mime_type")
                    if isinstance(data_b64, str) and data_b64:
                        return base64.b64decode(data_b64), mime

    # OpenAI-style image payload (best-effort)
    data = payload.get("data")
    if isinstance(data, list):
        for item in data:
            if isinstance(item, dict):
                b64 = item.get("b64_json") or item.get("image_base64")
                if isinstance(b64, str) and b64:
                    return base64.b64decode(b64), "image/png"

    output = payload.get("output")
    if isinstance(output, list):
        for item in output:
            if not isinstance(item, dict):
                continue
            content = item.get("content")
            if not isinstance(content, list):
                continue
            for block in content:
                if not isinstance(block, dict):
                    continue
                image_b64 = block.get("image_base64") or block.get("b64_json")
                if isinstance(image_b64, str) and image_b64:
                    return base64.b64decode(image_b64), "image/png"

    top_level = payload.get("image_base64")
    if isinstance(top_level, str) and top_level:
        return base64.b64decode(top_level), "image/png"

    raise ValueError("API response did not include any image payload")


def call_remaster_api(
    client: Any,
    api_mode: str,
    endpoint_template: str,
    model: str,
    api_key: str,
    prompt: str,
    png_bytes: bytes,
    timeout_seconds: float,
    max_retries: int,
) -> bytes:
    endpoint = endpoint_template.format(model=model)
    headers = {"Content-Type": "application/json"}
    params: Dict[str, str] = {}

    if api_mode == "gemini":
        # Gemini-style authentication.
        params["key"] = api_key
        payload = {
            "contents": [
                {
                    "parts": [
                        {"text": prompt},
                        {
                            "inlineData": {
                                "mimeType": "image/png",
                                "data": base64.b64encode(png_bytes).decode("ascii"),
                            }
                        },
                    ]
                }
            ],
            "generationConfig": {
                "responseModalities": ["IMAGE"],
            },
        }
    elif api_mode == "openai":
        headers["Authorization"] = f"Bearer {api_key}"
        payload = {
            "model": model,
            "input": [
                {
                    "role": "user",
                    "content": [
                        {"type": "input_text", "text": prompt},
                        {
                            "type": "input_image",
                            "image_url": f"data:image/png;base64,{base64.b64encode(png_bytes).decode('ascii')}",
                        },
                    ],
                }
            ],
            "modalities": ["image"],
        }
    else:
        raise ValueError(f"Unsupported api_mode: {api_mode}")

    # Ensure per-call timeout can override client defaults.
    timeout = httpx.Timeout(timeout_seconds) if httpx is not None else None

    for attempt in range(max_retries + 1):
        try:
            response = client.post(
                endpoint,
                params=params,
                headers=headers,
                json=payload,
                timeout=timeout,
            )

            if response.status_code in {429, 500, 502, 503, 504}:
                if attempt < max_retries:
                    backoff = (2**attempt) + random.uniform(0.0, 0.75)
                    logging.warning(
                        "API transient error %s, retrying in %.2fs (attempt %d/%d)",
                        response.status_code,
                        backoff,
                        attempt + 1,
                        max_retries,
                    )
                    time.sleep(backoff)
                    continue

            response.raise_for_status()

            content_type = response.headers.get("content-type", "").lower()
            if content_type.startswith("image/"):
                return response.content

            body = response.json()
            image_bytes, _mime = extract_image_bytes_from_response(body)
            return image_bytes
        except (httpx.TimeoutException, httpx.NetworkError) as exc:
            if attempt < max_retries:
                backoff = (2**attempt) + random.uniform(0.0, 0.75)
                logging.warning(
                    "API network/timeout error (%s), retrying in %.2fs (attempt %d/%d)",
                    exc,
                    backoff,
                    attempt + 1,
                    max_retries,
                )
                time.sleep(backoff)
                continue
            raise

    raise RuntimeError("Unexpected retry loop exit")


def collect_target_image_indices(payload: Dict[str, Any], only_base_color: bool) -> Set[int]:
    images = payload.get("images")
    if not isinstance(images, list) or not images:
        return set()

    if not only_base_color:
        return set(range(len(images)))

    textures = payload.get("textures")
    materials = payload.get("materials")
    if not isinstance(textures, list) or not isinstance(materials, list):
        return set(range(len(images)))

    texture_indices: Set[int] = set()
    for material in materials:
        if not isinstance(material, dict):
            continue
        pbr = material.get("pbrMetallicRoughness")
        if not isinstance(pbr, dict):
            continue
        base_color = pbr.get("baseColorTexture")
        if not isinstance(base_color, dict):
            continue
        texture_index = base_color.get("index")
        if not isinstance(texture_index, int):
            continue
        if texture_index < 0 or texture_index >= len(textures):
            continue
        texture = textures[texture_index]
        if not isinstance(texture, dict):
            continue
        image_index = texture.get("source")
        if isinstance(image_index, int) and 0 <= image_index < len(images):
            texture_indices.add(image_index)

    # If we cannot resolve any baseColor images, fallback to all textures.
    if not texture_indices:
        return set(range(len(images)))
    return texture_indices


def ensure_buffer_structures(payload: Dict[str, Any], initial_bin_len: int) -> List[Dict[str, Any]]:
    buffers = payload.get("buffers")
    if not isinstance(buffers, list) or not buffers:
        payload["buffers"] = [{"byteLength": initial_bin_len}]
        buffers = payload["buffers"]

    first_buffer = buffers[0]
    if not isinstance(first_buffer, dict):
        first_buffer = {"byteLength": initial_bin_len}
        buffers[0] = first_buffer

    first_buffer["byteLength"] = initial_bin_len
    first_buffer.pop("uri", None)

    buffer_views = payload.get("bufferViews")
    if not isinstance(buffer_views, list):
        payload["bufferViews"] = []
        buffer_views = payload["bufferViews"]

    return buffer_views


def append_buffer_view(
    buffer_views: List[Dict[str, Any]],
    binary_blob: bytearray,
    data: bytes,
) -> int:
    aligned_offset = align4(len(binary_blob))
    if aligned_offset > len(binary_blob):
        binary_blob.extend(b"\x00" * (aligned_offset - len(binary_blob)))

    byte_offset = len(binary_blob)
    binary_blob.extend(data)

    view = {
        "buffer": 0,
        "byteOffset": byte_offset,
        "byteLength": len(data),
    }
    buffer_views.append(view)
    return len(buffer_views) - 1


def save_raw_texture(
    raw_root: Path,
    rel_glb: Path,
    texture: TextureSource,
) -> Path:
    target_dir = raw_root / rel_glb.with_suffix("")
    target_dir.mkdir(parents=True, exist_ok=True)

    ext = extension_from_mime(texture.mime_type)
    base_name = safe_texture_name(Path(texture.logical_name).stem)
    filename = f"{texture.image_index:03d}_{base_name}{ext}"
    out_path = target_dir / filename
    out_path.write_bytes(texture.raw_bytes)
    return out_path


def process_glb(
    glb_path: Path,
    input_dir: Path,
    raw_dir: Path,
    out_dir: Path,
    args: argparse.Namespace,
    client: Optional[Any],
) -> FileReport:
    rel_glb = glb_path.relative_to(input_dir)
    report = FileReport(rel_glb=rel_glb)

    payload, binary_blob = load_glb_payload(glb_path)
    images = payload.get("images")

    out_path = out_dir / rel_glb
    out_path.parent.mkdir(parents=True, exist_ok=True)

    if not isinstance(images, list) or not images:
        logging.info("%s: no images found, copying GLB", rel_glb.as_posix())
        if not args.dry_run:
            out_path.write_bytes(glb_path.read_bytes())
        return report

    report.images_total = len(images)
    target_indices = collect_target_image_indices(payload, args.only_base_color)

    buffer_views = ensure_buffer_structures(payload, len(binary_blob))
    new_binary_blob = bytearray(binary_blob)

    replacements: Dict[int, Tuple[bytes, str]] = {}
    sources: Dict[int, TextureSource] = {}

    for image_index, image_obj in enumerate(images):
        if not isinstance(image_obj, dict):
            report.extraction_failed += 1
            logging.warning("%s: image[%d] is not an object", rel_glb.as_posix(), image_index)
            continue

        try:
            texture_source = extract_image_source(
                image_index=image_index,
                image_obj=image_obj,
                payload=payload,
                binary_blob=binary_blob,
                glb_path=glb_path,
            )
            sources[image_index] = texture_source
        except Exception as exc:  # noqa: BLE001
            report.extraction_failed += 1
            logging.warning(
                "%s: failed to extract image[%d]: %s",
                rel_glb.as_posix(),
                image_index,
                exc,
            )
            continue

    for image_index in sorted(sources):
        source = sources[image_index]

        if image_index not in target_indices:
            report.skipped_not_target += 1
            continue

        report.targeted_images += 1

        try:
            prepared = prepare_texture(source.raw_bytes)
        except Exception as exc:  # noqa: BLE001
            report.extraction_failed += 1
            logging.warning(
                "%s: image[%d] cannot be prepared for remaster: %s",
                rel_glb.as_posix(),
                image_index,
                exc,
            )
            continue

        if prepared.has_transparency:
            report.skipped_transparent += 1
            logging.debug(
                "%s: image[%d] skipped due to transparency",
                rel_glb.as_posix(),
                image_index,
            )
            continue

        if args.dry_run:
            logging.info(
                "[DRY-RUN] %s: image[%d] would remaster (%dx%d)",
                rel_glb.as_posix(),
                image_index,
                prepared.width,
                prepared.height,
            )
            continue

        save_raw_texture(raw_dir, rel_glb, source)

        report.attempted_requests += 1
        try:
            remastered_bytes = call_remaster_api(
                client=client,
                api_mode=args.api_mode,
                endpoint_template=args.endpoint_template,
                model=args.model,
                api_key=args.api_key,
                prompt=args.prompt,
                png_bytes=prepared.upload_png_bytes,
                timeout_seconds=args.timeout_seconds,
                max_retries=args.max_retries,
            )
            jpeg_bytes = encode_jpeg(remastered_bytes, quality=args.jpeg_quality)
            replacements[image_index] = (jpeg_bytes, "image/jpeg")
            report.remastered += 1
        except Exception as exc:  # noqa: BLE001
            report.api_failed += 1
            logging.warning(
                "%s: image[%d] remaster failed: %s",
                rel_glb.as_posix(),
                image_index,
                exc,
            )

    # Rebind changed images and embed any URI-based images so output GLB is self-contained.
    changed_any = False
    for image_index, source in sources.items():
        image_obj = images[image_index]

        replacement = replacements.get(image_index)
        should_embed_uri = source.source_kind in {"external_uri", "data_uri"}

        if replacement is None and not should_embed_uri:
            continue

        if replacement is not None:
            data_bytes, mime_type = replacement
        else:
            data_bytes = source.raw_bytes
            mime_type = source.mime_type or guess_mime_from_name(source.logical_name)

        new_view_index = append_buffer_view(buffer_views, new_binary_blob, data_bytes)
        image_obj["bufferView"] = new_view_index
        image_obj.pop("uri", None)
        if mime_type:
            image_obj["mimeType"] = mime_type

        changed_any = True

    if args.dry_run:
        return report

    if changed_any:
        buffers = payload["buffers"]
        first = buffers[0]
        first["byteLength"] = len(new_binary_blob)

        out_data = build_glb(payload, bytes(new_binary_blob))
        out_path.write_bytes(out_data)
    else:
        # Nothing changed in payload/binary: still mirror to remaster output tree.
        out_path.write_bytes(glb_path.read_bytes())

    return report


def aggregate(global_report: RunReport, file_report: FileReport, success: bool) -> None:
    global_report.files_total += 1
    if success:
        global_report.files_ok += 1
    else:
        global_report.files_failed += 1

    global_report.images_total += file_report.images_total
    global_report.targeted_images += file_report.targeted_images
    global_report.attempted_requests += file_report.attempted_requests
    global_report.remastered += file_report.remastered
    global_report.skipped_transparent += file_report.skipped_transparent
    global_report.skipped_not_target += file_report.skipped_not_target
    global_report.extraction_failed += file_report.extraction_failed
    global_report.api_failed += file_report.api_failed


def main(argv: Iterable[str]) -> int:
    args = parse_args(argv)
    configure_logging(args.verbose)

    if args.api_mode == "openai" and args.endpoint_template == DEFAULT_GEMINI_ENDPOINT:
        args.endpoint_template = DEFAULT_OPENAI_ENDPOINT

    if Image is None:
        logging.error("Missing dependency: Pillow. Install with `pip install Pillow`.")
        return 2

    if httpx is None and not args.dry_run:
        logging.error("Missing dependency: httpx. Install with `pip install httpx`.")
        return 2

    input_dir = args.input_dir.resolve()
    raw_dir = args.raw_dir.resolve()
    out_dir = args.out_dir.resolve()

    if not input_dir.exists() or not input_dir.is_dir():
        logging.error("Input directory does not exist or is not a directory: %s", input_dir)
        return 2

    glb_files = sorted(input_dir.rglob("*.glb"), key=lambda p: p.as_posix().lower())
    if not glb_files:
        logging.warning("No .glb files found under %s", input_dir)
        return 0

    if not args.dry_run and not args.api_key:
        logging.error(
            "API key is required. Use --api-key or env NANO_BANANA_API_KEY/GEMINI_API_KEY.",
        )
        return 2

    if not args.dry_run:
        raw_dir.mkdir(parents=True, exist_ok=True)
        out_dir.mkdir(parents=True, exist_ok=True)

    logging.info("Input dir: %s", input_dir)
    logging.info("Raw texture backup dir: %s", raw_dir)
    logging.info("Output GLB dir: %s", out_dir)
    logging.info("Found %d GLB files", len(glb_files))
    if args.dry_run:
        logging.info("Running in dry-run mode (no writes, no API calls)")

    run_report = RunReport()

    if args.dry_run:
        client = None
        for glb_path in glb_files:
            rel = glb_path.relative_to(input_dir)
            logging.info("Processing %s", rel.as_posix())
            try:
                file_report = process_glb(
                    glb_path=glb_path,
                    input_dir=input_dir,
                    raw_dir=raw_dir,
                    out_dir=out_dir,
                    args=args,
                    client=client,
                )
                aggregate(run_report, file_report, success=True)
            except Exception as exc:  # noqa: BLE001
                logging.exception("Failed processing %s: %s", rel.as_posix(), exc)
                aggregate(run_report, FileReport(rel_glb=rel), success=False)
    else:
        with httpx.Client() as client:
            for glb_path in glb_files:
                rel = glb_path.relative_to(input_dir)
                logging.info("Processing %s", rel.as_posix())
                try:
                    file_report = process_glb(
                        glb_path=glb_path,
                        input_dir=input_dir,
                        raw_dir=raw_dir,
                        out_dir=out_dir,
                        args=args,
                        client=client,
                    )
                    aggregate(run_report, file_report, success=True)
                except Exception as exc:  # noqa: BLE001
                    logging.exception("Failed processing %s: %s", rel.as_posix(), exc)
                    aggregate(run_report, FileReport(rel_glb=rel), success=False)

    logging.info("---- Remaster Summary ----")
    logging.info("Files: %d total | %d ok | %d failed", run_report.files_total, run_report.files_ok, run_report.files_failed)
    logging.info("Images discovered: %d", run_report.images_total)
    logging.info("Images targeted: %d", run_report.targeted_images)
    logging.info("Requests attempted: %d", run_report.attempted_requests)
    logging.info("Images remastered: %d", run_report.remastered)
    logging.info("Skipped (transparent): %d", run_report.skipped_transparent)
    logging.info("Skipped (not target set): %d", run_report.skipped_not_target)
    logging.info("Extraction/prep failures: %d", run_report.extraction_failed)
    logging.info("API failures: %d", run_report.api_failed)

    return 1 if run_report.files_failed else 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
