#!/usr/bin/env bash
set -euo pipefail
out=$1
mkdir -p "$out"
for target in x86_64-unknown-linux-musl x86_64-unknown-linux-gnu; do
  features=()
  if [[ $target == *-gnu ]]; then features=(--features credential-store); fi
  cargo build --locked --release -p zixcel-microsoft-cli --bin zix-microsoft --target "$target" "${features[@]}"
  cargo run --locked -p zixcel-microsoft-cli --bin package-native -- "$PWD/target/$target/release/zix-microsoft" "$out/$target" "$target"
  (cd "$out/$target"; sha256sum --check SHA256SUMS)
  mkdir "$out/$target/unpacked"
  tar -xzf "$out/$target/"*.tar.gz -C "$out/$target/unpacked"
  (
    cd "$out/$target/unpacked/"*
    sha256sum --check CONTENTS.sha256
    ./zix-microsoft --manifest > observed.json
    cmp observed.json microsoft.extension.json
    if [[ $target == *-gnu ]]; then
      test -d THIRD-PARTY/libsecret-0.9.0
      test -d THIRD-PARTY/libsecret-sys-0.9.0
      test -d THIRD-PARTY/crowsi-credential-broker-0.1.2
    fi
  )
  cp "$out/$target/"*.tar.gz "$out/"
done
(cd "$out"; sha256sum ./*.tar.gz | sed 's|  ./|  |' > SHA256SUMS; sha256sum --check SHA256SUMS)
