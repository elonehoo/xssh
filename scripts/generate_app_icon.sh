#!/usr/bin/env bash
set -euo pipefail

repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
source_image="${1:-"$repo_root/assets/app-icon/source.png"}"
output_dir="${2:-"$repo_root/assets/app-icon"}"
iconset_dir="$output_dir/AppIcon.iconset"
base_icon="$output_dir/app-icon-1024.png"
icns_file="$output_dir/app-icon.icns"

if [[ ! -f "$source_image" ]]; then
  echo "Source image not found: $source_image" >&2
  exit 1
fi

if ! command -v sips >/dev/null 2>&1; then
  echo "sips is required on macOS" >&2
  exit 1
fi

if ! command -v iconutil >/dev/null 2>&1; then
  echo "iconutil is required on macOS" >&2
  exit 1
fi

mkdir -p "$iconset_dir"

sips -s format png -z 1024 1024 "$source_image" --out "$base_icon" >/dev/null

icon_specs=(
  "16 icon_16x16.png"
  "32 icon_16x16@2x.png"
  "32 icon_32x32.png"
  "64 icon_32x32@2x.png"
  "128 icon_128x128.png"
  "256 icon_128x128@2x.png"
  "256 icon_256x256.png"
  "512 icon_256x256@2x.png"
  "512 icon_512x512.png"
  "1024 icon_512x512@2x.png"
)

for spec in "${icon_specs[@]}"; do
  read -r size filename <<<"$spec"
  sips -s format png -z "$size" "$size" "$base_icon" --out "$iconset_dir/$filename" >/dev/null
done

iconutil -c icns "$iconset_dir" -o "$icns_file"

echo "Generated:"
echo "  $base_icon"
echo "  $iconset_dir"
echo "  $icns_file"
