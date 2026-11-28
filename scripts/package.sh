#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
task_package_dir="$(pwd)/target/package"
mkdir -p "$task_package_dir"
cp packaging/PKGBUILD "$task_package_dir/PKGBUILD"
tar -czf "$task_package_dir/omusic-0.1.0.tar.gz" --transform='s,^,omusic-0.1.0/,' \
    Cargo.toml Cargo.lock rust-toolchain.toml src packaging README.md LICENSE
cd "$task_package_dir"
exec makepkg --force "$@"
