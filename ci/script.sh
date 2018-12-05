#!/usr/bin/env bash
set -ex

cargo=cargo
target_param=""
features=""
if [ ! -z "$UNSTABLE" ]; then
    features+=" unstable"
fi
if [ ! -z "$TARGET" ]; then
    rustup target add "$TARGET"
    cargo install cross --force
    cargo="cross"
    target_param="--target $TARGET"
fi

$cargo build -v $target_param --features "$features"
if [ "$TRAVIS_RUST_VERSION" = "1.13.0" ]; then
    # unfortunately, testing requires building dev-deps, which
    # requires a newer rustc than this.
    exit 0
fi

$cargo test -v $target_param --features "$features"

# for now, `cross bench` is broken https://github.com/rust-embedded/cross/issues/239
if [ "$cargo" != "cross" ]; then
    $cargo bench -v $target_param --features "$features" -- --test # don't actually record numbers
fi
$cargo doc -v $target_param --features "$features"


if [ ! -z "$COVERAGE" ]; then
    if [ ! -z "$TARGET" ]; then
        echo "cannot record coverage while cross compiling"
        exit 1
    fi

    cargo install -v cargo-travis --force
    cargo coveralls -v --features "$features"
fi
