#!/usr/bin/env python3
"""Convert Kenney's sedan.glb to the ROADOSMF model file task 38 defines.

What it is: an offline converter. It reads the pinned GLB out of
.asset-cache/, bakes node translations into the vertices, concatenates the
five meshes in model.json's merge_order, and writes the interleaved file
ui/src/ui_demo/assets/sedan.roados plus a byte copy of the colormap.

What it reads: tools/asset-pipeline/model.json (the manifest: paths, the
merge order, and the measured block every number below is checked against)
and .asset-cache/kenney_car-kit/.../sedan.glb.

What it writes: ui/src/ui_demo/assets/sedan.roados and
ui/src/ui_demo/assets/colormap.png.

What it must not be used for: converting any other model (a different mesh
needs its own manifest and its own measured decisions), converting a subset
of this one (there is no --only/--mesh/--part flag on purpose), or running
inside the demo (nothing produced here runs inside ui_demo).

Layout reference: doc/ui/TASK_UI_PRIM_38.md requirement 2 owns the byte
table; this tool emits it and does not restate it. The table is also
verbatim in ui_core render/meshio.rs's module doc, which is the copy a
reviewer diffs --print-layout against.
"""

import argparse
import copy
import json
import os
import shutil
import struct
import sys

try:
    import numpy as np
except ImportError:
    print("glb_to_model: numpy is required (python3 with numpy; see "
          "tools/asset-pipeline/README.md). No numpy, no conversion.",
          file=sys.stderr)
    sys.exit(1)

HEADER_FORMAT = "<8s8I"
HEADER_BYTES = 40
VERTEX_STRIDE = 32

COMPONENT_DTYPES = {5121: "<u1", 5123: "<u2", 5125: "<u4", 5126: "<f4"}
TYPE_LANES = {"SCALAR": 1, "VEC2": 2, "VEC3": 3, "VEC4": 4}


def fail(message):
    print("glb_to_model: error: " + message, file=sys.stderr)
    sys.exit(1)


def read_bytes(blob, view, accessor, what):
    """Read one accessor honouring byteOffset and byteStride.

    In this file both fields are harmless (no accessor byteOffset, and every
    strided view has byteStride 12, the natural size of a VEC3 of f32), and
    a packed reader would coincidentally produce correct output here. The
    reader honours both anyway: the next asset is not this one, and a
    silently mis-strided NORMAL is still unit-length, so it would change
    the shading and raise nothing.
    """
    ctype = accessor["componentType"]
    lanes = TYPE_LANES[accessor["type"]]
    item = np.dtype(COMPONENT_DTYPES[ctype]).itemsize
    natural = item * lanes
    stride = view.get("byteStride", natural)
    base = view.get("byteOffset", 0) + accessor.get("byteOffset", 0)
    count = accessor["count"]
    out = np.empty(count * lanes, dtype=np.dtype(COMPONENT_DTYPES[ctype]))
    for row in range(count):
        start = base + row * stride
        chunk = blob[start:start + natural]
        if len(chunk) != natural:
            fail("%s: accessor overruns its bufferView" % what)
        out[row * lanes:(row + 1) * lanes] = np.frombuffer(chunk, dtype=out.dtype)
    return out.reshape(count, lanes) if lanes > 1 else out


def load_glb(path):
    with open(path, "rb") as handle:
        blob = handle.read()
    if len(blob) < 12 or blob[0:4] != b"glTF":
        fail("%s: not a GLB file" % path)
    _magic, version, _length = struct.unpack("<III", blob[0:12])
    if version != 2:
        fail("%s: GLB version %d, only 2 is read" % (path, version))
    cursor = 12
    chunks = {}
    while cursor + 8 <= len(blob):
        size, kind = struct.unpack("<II", blob[cursor:cursor + 8])
        body = blob[cursor + 8:cursor + 8 + size]
        if len(body) != size:
            fail("%s: chunk overruns the file" % path)
        chunks[kind] = body
        cursor += 8 + size
    if 0x4E4F534A not in chunks or 0x004E4942 not in chunks:
        fail("%s: need a JSON chunk and a BIN chunk" % path)
    try:
        document = json.loads(chunks[0x4E4F534A].decode("utf-8"))
    except (UnicodeDecodeError, ValueError) as error:
        fail("%s: JSON chunk does not parse: %s" % (path, error))
    return document, chunks[0x004E4942]


def check_accessor(document, index, what):
    accessors = document.get("accessors", [])
    views = document.get("bufferViews", [])
    if not (0 <= index < len(accessors)):
        fail("%s: accessor index %d out of range" % (what, index))
    accessor = accessors[index]
    if accessor["componentType"] not in COMPONENT_DTYPES:
        fail("%s: unsupported componentType %s" % (what, accessor["componentType"]))
    if accessor["type"] not in TYPE_LANES:
        fail("%s: unsupported type %s" % (what, accessor["type"]))
    lanes = TYPE_LANES[accessor["type"]]
    for bound in ("min", "max"):
        if bound in accessor and len(accessor[bound]) != lanes:
            fail("%s: accessor count disagrees with its %s" % (what, bound))
    view_index = accessor.get("bufferView")
    if view_index is None or not (0 <= view_index < len(views)):
        fail("%s: accessor has no usable bufferView" % what)
    return accessor, views[view_index]


def read_primitive(document, blob, mesh_name, mesh_index, primitive):
    if primitive.get("mode", 4) != 4:
        fail("mesh %s (index %d): mode is not TRIANGLES" % (mesh_name, mesh_index))
    attributes = primitive.get("attributes", {})
    for field in ("POSITION", "NORMAL", "TEXCOORD_0"):
        if field not in attributes:
            fail("mesh %s (index %d): primitive has no %s" % (mesh_name, mesh_index, field))
    arrays = {}
    for field in ("POSITION", "NORMAL", "TEXCOORD_0", "TANGENT"):
        if field not in attributes:
            continue
        accessor, view = check_accessor(
            document, attributes[field], "mesh %s attribute %s" % (mesh_name, field))
        if accessor["componentType"] != 5126:
            fail("mesh %s (index %d): %s componentType is not FLOAT"
                 % (mesh_name, mesh_index, field))
        if accessor.get("normalized", False):
            fail("mesh %s (index %d): %s is normalized" % (mesh_name, mesh_index, field))
        arrays[field] = read_bytes(
            blob, view, accessor, "mesh %s attribute %s" % (mesh_name, field))
    # TANGENT is read and discarded, never carried: MeshVertex is three
    # fields at stride 32 and carrying one would be a stride change, which
    # task 38 puts on whoever changes the format.
    arrays.pop("TANGENT", None)
    if "indices" not in primitive:
        fail("mesh %s (index %d): primitive has no indices" % (mesh_name, mesh_index))
    accessor, view = check_accessor(
        document, primitive["indices"], "mesh %s indices" % mesh_name)
    if accessor["componentType"] not in (5121, 5123):
        fail("mesh %s (index %d): index componentType is neither UNSIGNED_BYTE "
             "nor UNSIGNED_SHORT" % (mesh_name, mesh_index))
    # Indices widen u16 -> u32 on the way out (task 35's buffers are
    # GL_UNSIGNED_INT); there is no u32 -> u16 narrowing anywhere here.
    arrays["INDICES"] = read_bytes(
        blob, view, accessor, "mesh %s indices" % mesh_name).astype(np.int64)
    return arrays


def check_texture_transform(document):
    for number, material in enumerate(document.get("materials", [])):
        transform = (((material.get("extensions") or {}).get("KHR_texture_transform")) or
                     ((((material.get("pbrMetallicRoughness") or {}).get("baseColorTexture")
                        or {}).get("extensions") or {}).get("KHR_texture_transform")))
        if transform is None:
            continue
        if "offset" in transform and list(transform["offset"]) != [0.0, 0.0]:
            fail("material %d: KHR_texture_transform offset is not the identity" % number)
        if "scale" in transform and list(transform["scale"]) != [1.0, 1.0]:
            fail("material %d: KHR_texture_transform scale is not the identity" % number)


def find_nodes(document):
    scenes = document.get("scenes", [])
    if not scenes:
        fail("no scenes in the GLB")
    scene = scenes[document.get("scene", 0)]
    nodes = document.get("nodes", [])
    by_mesh = {}
    for node_index in scene.get("nodes", []):
        if not (0 <= node_index < len(nodes)):
            fail("scene references node index %d out of range" % node_index)
        node = nodes[node_index]
        if "mesh" not in node:
            continue
        mesh_index = node["mesh"]
        if not (0 <= mesh_index < len(document.get("meshes", []))):
            fail("node %d references mesh index %d out of range" % (node_index, mesh_index))
        by_mesh.setdefault(mesh_index, []).append((node_index, node))
    return by_mesh


def bake_translation(name, positions, normals, node):
    """Apply a node's translation; refuse anything more general.

    A GLB separates mesh geometry from node placement: the wheels are
    authored at the origin and moved by their node's translation, so reading
    meshes alone puts all four wheels inside the body. All five nodes here
    carry only a translation (the body's [0.0, 0.15, -0.025] included — a
    converter that special-cased the wheels would get the ride height wrong
    by 15 cm with no message), and anything else is refused naming the node
    rather than silently ignored into a slightly-wrong car.
    """
    if node.get("matrix") is not None:
        fail("node for %s carries a matrix; refusing rather than ignoring it" % name)
    if node.get("rotation") is not None:
        fail("node for %s carries a rotation; refusing rather than ignoring it" % name)
    scale = node.get("scale", [1.0, 1.0, 1.0])
    if list(scale) != [1.0, 1.0, 1.0]:
        fail("node for %s carries a non-unit scale; refusing" % name)
    translation = np.asarray(node.get("translation", [0.0, 0.0, 0.0]), dtype=np.float32)
    # Positions are points (w = 1): they move. Normals are directions
    # (w = 0): a translation must not move them; they are renormalised
    # belt-and-braces, never recomputed from faces.
    moved = (positions.astype(np.float32) + translation).astype(np.float32)
    lengths = np.linalg.norm(normals.astype(np.float64), axis=1, keepdims=True)
    lengths[lengths == 0.0] = 1.0
    kept = (normals.astype(np.float64) / lengths).astype(np.float32)
    return moved, kept


def weld_key(positions, decimals):
    table = {}
    for number, row in enumerate(np.round(positions.astype(np.float64), decimals)):
        table.setdefault(tuple(row), []).append(number)
    return table


def check_winding(name, positions, normals, flat_indices):
    """Every triangle's face-versus-vertex dots are >= 0.0.

    A zero-area triangle agrees vacuously: it emits no fragments under
    either winding, so its winding is irrelevant. The single
    near-degenerate sliver (body tri 588, area 4.5e-9, dots exactly 0.0)
    passes this rule and would fail a strict > 0.0 one; the six
    exactly-degenerate body tris pass it vacuously.
    """
    total = 0
    agreeing = 0
    for a, b, c in flat_indices.reshape(-1, 3):
        total += 1
        edge1 = positions[b].astype(np.float64) - positions[a].astype(np.float64)
        edge2 = positions[c].astype(np.float64) - positions[a].astype(np.float64)
        face = np.cross(edge1, edge2)
        length = float(np.linalg.norm(face))
        if length == 0.0:
            agreeing += 1
            continue
        face = face / length
        dots = [float(np.dot(face, normals[v].astype(np.float64))) for v in (a, b, c)]
        if min(dots) >= 0.0:
            agreeing += 1
        else:
            print("glb_to_model: winding: mesh %s triangle disagrees: %s"
                  % (name, ["%.4f" % d for d in dots]), file=sys.stderr)
    return agreeing, total


def read_model(root, manifest):
    """Read the GLB subset and return (per_mesh, whole) measurements.

    check_assets.py imports this function so the guard measures with the
    same reader that wrote the file: two readers would be two things to
    keep honest. Per-mesh entries carry baked positions, kept normals, uvs
    and flat u32-ready indices; whole carries the manifest's measured block
    as read off the file.
    """
    cache_glb = os.path.join(root, ".asset-cache", "kenney_car-kit",
                             manifest["model"].replace("\\", "/"))
    if not os.path.isfile(cache_glb):
        fail("GLB not in cache: %s (run fetch_upstream.sh)" % cache_glb)
    document, blob = load_glb(cache_glb)
    check_texture_transform(document)
    by_mesh = find_nodes(document)
    meshes = document.get("meshes", [])
    for mesh_index, mesh in enumerate(meshes):
        if mesh_index not in by_mesh:
            fail("mesh %s (index %d) is referenced by no node in the scene; "
                 "an orphan the scene walk would drop silently" % (mesh.get("name"), mesh_index))
        if len(by_mesh[mesh_index]) != 1:
            fail("mesh %s (index %d) is referenced by %d nodes; the bake "
                 "would be ambiguous" % (mesh.get("name"), mesh_index, len(by_mesh[mesh_index])))
    order = manifest["merge_order"]
    names = [mesh.get("name") for mesh in meshes]
    for name in order:
        if name not in names:
            fail("merge_order names %s, which is not a mesh in the GLB" % name)
    per_mesh = {}
    for name in order:
        mesh_index = names.index(name)
        mesh = meshes[mesh_index]
        primitives = mesh.get("primitives", [])
        if len(primitives) != 1:
            fail("mesh %s: need one primitive, found %d" % (name, len(primitives)))
        arrays = read_primitive(document, blob, name, mesh_index, primitives[0])
        _node_index, node = by_mesh[mesh_index][0]
        baked, kept = bake_translation(
            name, arrays["POSITION"], arrays["NORMAL"], node)
        per_mesh[name] = {
            "positions": baked,
            "normals": kept,
            "uvs": arrays["TEXCOORD_0"].astype(np.float32),
            "indices": arrays["INDICES"].reshape(-1),
            "translation": [float(v) for v in node.get("translation", [0.0, 0.0, 0.0])],
        }
    whole = {
        "meshes": len(meshes),
        "triangles": sum(len(v["indices"]) // 3 for v in per_mesh.values()),
        "triangles_per_mesh": {n: len(v["indices"]) // 3 for n, v in per_mesh.items()},
        "vertices": sum(len(v["positions"]) for v in per_mesh.values()),
        "indices": sum(len(v["indices"]) for v in per_mesh.values()),
    }
    return per_mesh, whole, document


def measure_against_manifest(manifest, per_mesh, document):
    """Compare what the GLB holds against model.json's measured block.

    A measured block that disagrees with what the tool reads is a hard
    failure naming the field and both values: change the upstream model
    and the pipeline refuses rather than quietly emitting a different car.
    """
    measured = manifest["measured"]
    failures = []

    def check(field, read, recorded):
        if read != recorded:
            failures.append("%s: tool reads %r, manifest records %r" % (field, read, recorded))

    check("meshes", len(per_mesh), measured["meshes"])
    check("triangles", sum(len(v["indices"]) // 3 for v in per_mesh.values()),
          measured["triangles"])
    check("triangles_per_mesh",
          {n: len(v["indices"]) // 3 for n, v in per_mesh.items()},
          measured["triangles_per_mesh"])
    check("vertices", sum(len(v["positions"]) for v in per_mesh.values()),
          measured["vertices"])
    check("indices", sum(len(v["indices"]) for v in per_mesh.values()),
          measured["indices"])
    check("glb_mesh_order", [m.get("name") for m in document.get("meshes", [])],
          measured["glb_mesh_order"])
    check("node_translations",
          {n: v["translation"] for n, v in per_mesh.items()},
          measured["node_translations"])
    if document.get("asset", {}).get("generator") != "UnityGLTF":
        failures.append("generator: tool reads %r" % (document.get("asset", {}).get("generator"),))

    index_type = None
    for mesh in document.get("meshes", []):
        accessor = document["accessors"][mesh["primitives"][0]["indices"]]
        index_type = accessor["componentType"]
    check("index_component_type", index_type, measured["index_component_type"])

    sampler = (document.get("samplers", []) or [{}])[0]
    check("min_filter", sampler.get("minFilter"), measured["min_filter"])
    material = (document.get("materials", []) or [{}])[0]
    check("double_sided", material.get("doubleSided"), measured["double_sided"])

    degenerate = {}
    winding_agree = 0
    winding_total = 0
    uv_inside = True
    for name, entry in per_mesh.items():
        positions = entry["positions"]
        flat = entry["indices"]
        tris = flat.reshape(-1, 3)
        v0 = positions[tris[:, 0]].astype(np.float64)
        v1 = positions[tris[:, 1]].astype(np.float64)
        v2 = positions[tris[:, 2]].astype(np.float64)
        areas = np.linalg.norm(np.cross(v1 - v0, v2 - v0), axis=1) / 2.0
        zeros = [int(i) for i in np.where(areas == 0.0)[0]]
        if zeros:
            degenerate[name] = zeros
        agree, total = check_winding(name, positions, entry["normals"], flat)
        winding_agree += agree
        winding_total += total
        uvs = entry["uvs"].astype(np.float64)
        if uvs.min() < 0.0 or uvs.max() > 1.0:
            uv_inside = False
    check("degenerate_triangles", sum(len(v) for v in degenerate.values()),
          measured["degenerate_triangles"])
    if "degenerate_triangle_indices" in measured:
        recorded = measured["degenerate_triangle_indices"]
        got = {n: v for n, v in degenerate.items()}
        if got != {n: list(v) for n, v in recorded.items()}:
            failures.append("degenerate_triangle_indices: tool reads %r, manifest records %r"
                            % (got, recorded))
    check("winding_agreement", "%d/%d" % (winding_agree, winding_total),
          measured["winding_agreement"])
    check("uv_within_unit_square", uv_inside, measured["uv_within_unit_square"])

    decimals = measured.get("weld_decimals", 3)
    closed = True
    for name, entry in per_mesh.items():
        table = weld_key(entry["positions"], decimals)
        canon = np.empty(len(entry["positions"]), dtype=np.int64)
        for number, members in enumerate(table.values()):
            canon[members] = number
        valence = {}
        for a, b, c in entry["indices"].reshape(-1, 3):
            for x, y in ((canon[a], canon[b]), (canon[b], canon[c]), (canon[c], canon[a])):
                key = (min(x, y), max(x, y))
                valence[key] = valence.get(key, 0) + 1
        histogram = {}
        for count in valence.values():
            histogram[count] = histogram.get(count, 0) + 1
        if set(histogram) != {2}:
            closed = False
            print("glb_to_model: weld: mesh %s valence histogram %r" % (name, histogram),
                  file=sys.stderr)
    check("shells_closed", closed, measured["shells_closed"])
    check("welded_edge_valence_2", closed, measured["welded_edge_valence_2"])

    merged = np.vstack([v["positions"] for v in [per_mesh[n] for n in manifest["merge_order"]]])
    bounds_min = [round(float(v), 6) for v in merged.min(axis=0)]
    bounds_max = [round(float(v), 6) for v in merged.max(axis=0)]
    for got, want, label in ((bounds_min, measured["bounds_min"], "bounds_min"),
                             (bounds_max, measured["bounds_max"], "bounds_max")):
        if any(abs(g - w) > 1e-4 for g, w in zip(got, want)):
            failures.append("%s: tool reads %r, manifest records %r" % (label, got, want))

    vertex_count = sum(len(v["positions"]) for v in per_mesh.values())
    index_count = sum(len(v["indices"]) for v in per_mesh.values())
    name_block = b"".join(n.encode("utf-8") + b"\0" for n in manifest["merge_order"])
    byte_block = {"header": 40, "vertices": vertex_count * 32, "indices": index_count * 4,
                  "sub_meshes": len(per_mesh) * 8, "names": len(name_block),
                  "total": 40 + vertex_count * 32 + index_count * 4 + len(per_mesh) * 8
                  + len(name_block)}
    check("bytes", byte_block, measured["bytes"])
    return failures


def emit_roados(path, manifest, per_mesh):
    names = manifest["merge_order"]
    vertex_blocks = []
    index_blocks = []
    pairs = []
    cursor = 0
    index_cursor = 0
    for name in names:
        entry = per_mesh[name]
        count = len(entry["positions"])
        flat = (entry["indices"].astype(np.int64) + cursor).astype("<u4")
        # NO u32 -> u16 narrowing on the way out: the file's indices are u32
        # and stay u32; a narrower type would need a range proof per mesh.
        if int(flat.max()) >= cursor + count:
            fail("mesh %s: an index escapes its own vertex array" % name)
        vertex = np.empty((count, 8), dtype="<f4")
        vertex[:, 0:3] = entry["positions"].astype("<f4")
        vertex[:, 3:6] = entry["normals"].astype("<f4")
        vertex[:, 6:8] = entry["uvs"].astype("<f4")
        vertex_blocks.append(vertex.tobytes())
        index_blocks.append(flat.tobytes())
        if len(flat) % 3 != 0:
            fail("mesh %s: index count is not a multiple of 3" % name)
        # first_index is an index position, not a byte offset and not a
        # vertex number: the vertex base above and this cursor advance by
        # different amounts (body: 1,072 vertices but 2,112 indices), and
        # sharing one cursor tiles the ranges wrong while keeping every
        # count right — the exact shape of a silent misparse.
        pairs.append((index_cursor, len(flat)))
        cursor += count
        index_cursor += len(flat)
    # NO re-indexing, vertex reordering, index sorting or deduplication: the
    # output is the concatenation, so two runs on two machines produce the
    # same bytes by construction rather than by a canonicalisation pass.
    vertices_blob = b"".join(vertex_blocks)
    indices_blob = b"".join(index_blocks)
    names_blob = b"".join(n.encode("utf-8") + b"\0" for n in names)
    header = struct.pack(HEADER_FORMAT, b"ROADOSMF", manifest["version"],
                         manifest["header_bytes"], cursor,
                         sum(p[1] for p in pairs), len(names), len(names_blob),
                         manifest["flags"], manifest["reserved"])
    # NO compression: 126,425 bytes read once at start-up; a compressor would
    # make task 38's validation a decompressor's problem and spend the flags
    # and reserved words that 38 reserves for a checksum.
    with open(path, "wb") as handle:
        handle.write(header)
        handle.write(vertices_blob)
        handle.write(indices_blob)
        for first, count in pairs:
            handle.write(struct.pack("<II", first, count))
        handle.write(names_blob)
    # NO welding: 528 wheel vertices are only 168 millimetre-distinct
    # positions and the body 1,072 only 358, with every welded edge at
    # valence exactly 2 — the duplication is how the export stores split
    # normals, not sloppy topology, and welding would collapse them.
    # NO tangent generation: MeshVertex is three fields at stride 32 and the
    # GLB's own TANGENT was read and discarded above; a tangent is a stride
    # change, which task 38 puts on whoever changes the format.
    # NO smoothing or normal averaging: the same evidence as welding, and
    # doing it after not welding is the same damage twice.
    # NO normal recomputation: the file's normals are already unit length on
    # all five meshes while the body's face-versus-vertex dot goes as low as
    # 0.0 — the body carries averaged normals across its creases, and
    # recomputing from faces would facet it. They are kept, renormalised.
    # NO degenerate-triangle removal: body keeps 6 zero-area triangles
    # (18 of 6,096 indices, 0.3 %) because removing them opens the shell —
    # welded valence goes {2: 1056} to {1: 14, 2: 1040}, fourteen holes in a
    # shell task 37 back-face-culls. They rasterise nothing; holes show.
    # NO winding normalisation: 2,032 of 2,032 agree under the >= 0.0 rule,
    # already matching GL_CCW with cull_face(GL_BACK). Normalising would
    # change bytes for no gain under a re-orientation rule nothing needs.
    print("glb_to_model: wrote %s (%d vertices, %d indices, %d sub-meshes)"
          % (path, cursor, sum(p[1] for p in pairs), len(names)))


def print_layout(manifest):
    measured = manifest["measured"]
    print("magic = b\"ROADOSMF\", version = 1, header_bytes = 40, flags = 0, reserved = 0")
    print("vertex_count = %d, index_count = %d, sub_mesh_count = %d, name_bytes = %d"
          % (measured["vertices"], measured["indices"], measured["meshes"],
             measured["bytes"]["names"]))
    print("blocks in order: interleaved vertices (stride 32: position[3], normal[3], uv[2]), "
          "u32 indices, (first_index, index_count) pairs, NUL-terminated names")
    print("merge_order = %s" % (" ".join(manifest["merge_order"]),))
    print("little-endian throughout, no padding, no bounding box, no pivot, "
          "no texture reference, no compression")
    print("colormap beside the model at %s (byte copy, not embedded)"
          % (manifest["out_texture"],))


def main(argv):
    parser = argparse.ArgumentParser(
        description="Convert Kenney's sedan.glb to the ROADOSMF model file. "
                    "Converts all five meshes or fails; there is deliberately "
                    "no way to ask for a part.")
    parser.add_argument("--manifest", required=True, help="path to model.json")
    parser.add_argument("--print-layout", action="store_true",
                        help="print task 38's table as model.json records it and exit")
    arguments = parser.parse_args(argv)
    with open(arguments.manifest, "r", encoding="utf-8") as handle:
        manifest = json.load(handle)
    if arguments.print_layout:
        print_layout(manifest)
        return 0
    root = os.path.dirname(os.path.dirname(os.path.dirname(
        os.path.abspath(arguments.manifest))))
    per_mesh, _whole, document = read_model(root, manifest)
    failures = measure_against_manifest(manifest, per_mesh, document)
    if failures:
        for failure in failures:
            print("glb_to_model: measured mismatch: " + failure, file=sys.stderr)
        return 1
    out_model = os.path.join(root, manifest["out_model"].replace("\\", "/"))
    os.makedirs(os.path.dirname(out_model), exist_ok=True)
    emit_roados(out_model, manifest, per_mesh)
    cache_texture = os.path.join(root, ".asset-cache", "kenney_car-kit",
                                 manifest["texture"].replace("\\", "/"))
    out_texture = os.path.join(root, manifest["out_texture"].replace("\\", "/"))
    # A byte copy, never re-encoded through Pillow: a re-encode is a byte
    # change for no benefit, and the copy is what makes colormap.png
    # byte-stable across runs and hosts.
    shutil.copyfile(cache_texture, out_texture)
    print("glb_to_model: copied %s" % (out_texture,))
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
