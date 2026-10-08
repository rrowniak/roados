# What this is
#
# Fetches the two pinned third-party upstreams the asset pipeline converts:
# Kenney's Car Kit (the sedan mesh) and Lucide (the icon SVGs). Reads nothing
# except this directory's upstream.sha256; writes only .asset-cache/ at the
# repository root (downloads plus the two extracted trees).
#
# What it reads: tools/asset-pipeline/upstream.sha256 (the two pinned hashes).
# What it writes: .asset-cache/<archive>.zip and .asset-cache/<tree>/.
#
# What it must not be used for: fetching anything not in its allow-list.
# Pointing this pipeline at a third icon set is how a trademarked
# car-maker mark (from a CC0 set that therefore looks safe) enters the
# repository. A new source is a one-line diff here, reviewed like any
# other — never a flag, never an edit made to keep going.
set -u

KENNEY_URL="https://kenney.nl/media/pages/assets/car-kit/1a312ec241-1775131960/kenney_car-kit.zip"
LUCIDE_URL="https://github.com/lucide-icons/lucide/archive/refs/tags/1.52.0.zip"

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
CACHE="$ROOT/.asset-cache"
SHAFILE="$ROOT/tools/asset-pipeline/upstream.sha256"

# Local file names are pinned here, not derived from the URLs: the Lucide
# tag URL ends in 1.52.0.zip, which names the version but not the project,
# so basename-from-URL would store, hash and extract under a name no other
# half of the pipeline recognises.
local_name() {
    case "$1" in
        "$KENNEY_URL") echo "kenney_car-kit.zip" ;;
        "$LUCIDE_URL") echo "lucide-1.52.0.zip" ;;
        *)
            echo "fetch_upstream: refusing URL not in the allow-list: $1" >&2
            echo "fetch_upstream: permitted URLs are:" >&2
            echo "fetch_upstream:   $KENNEY_URL" >&2
            echo "fetch_upstream:   $LUCIDE_URL" >&2
            return 1
            ;;
    esac
}

fetch_one() {
    url="$1"
    name="$(local_name "$url")" || return 1
    file="$CACHE/$name"
    want="$(grep -F "  $name" "$SHAFILE" | cut -d' ' -f1)"
    if [ -z "$want" ]; then
        echo "fetch_upstream: internal error: upstream.sha256 has no line for $name" >&2
        return 1
    fi
    if [ -f "$file" ]; then
        if (cd "$CACHE" && echo "$want  $name" | sha256sum -c - >/dev/null 2>&1); then
            echo "fetch_upstream: $file present with the recorded hash; running from cache, no download."
            return 0
        fi
        echo "fetch_upstream: $file present but the hash moved; re-downloading."
    fi
    if ! curl -sSfL --retry 3 -o "$file" "$url"; then
        echo "fetch_upstream: download failed for $url (no network and no cache in $CACHE). Nothing was extracted." >&2
        rm -f "$file"
        return 1
    fi
    if ! (cd "$CACHE" && echo "$want  $name" | sha256sum -c -); then
        echo "fetch_upstream: STOP — hash mismatch for $url." >&2
        echo "fetch_upstream: the upstream moved under a pinned URL. Re-read the licence," >&2
        echo "fetch_upstream: re-record the hash deliberately; do not edit upstream.sha256 and continue." >&2
        return 1
    fi
}

extract_one() {
    archive="$1"
    dir="$2"
    # A partial extraction is removed before the extraction begins, so an
    # interrupted run cannot leave a half-tree a later run treats as complete.
    rm -rf "$CACHE/$dir"
    # python3's zipfile, not the unzip binary: some hosts (this one included)
    # do not ship unzip, and python3 is already a pipeline dependency.
    if ! python3 -c "import zipfile,sys; zipfile.ZipFile(sys.argv[1]).extractall(sys.argv[2])" \
        "$CACHE/$archive" "$CACHE/$dir"; then
        echo "fetch_upstream: extraction failed for $archive; removing the half-tree." >&2
        rm -rf "$CACHE/$dir"
        return 1
    fi
}

mkdir -p "$CACHE"
fetch_one "$KENNEY_URL" || exit 1
fetch_one "$LUCIDE_URL" || exit 1
# URLs may also be passed explicitly, to demonstrate the refusal:
# every argument must be in the allow-list or the run fails here.
for arg in "$@"; do
    fetch_one "$arg" || exit 1
done
extract_one "kenney_car-kit.zip" "kenney_car-kit" || exit 1
extract_one "lucide-1.52.0.zip" "lucide-1.52.0" || exit 1
echo "fetch_upstream: both trees extracted under $CACHE."
