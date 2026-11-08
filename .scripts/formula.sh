#!/usr/bin/env bash
set -euo pipefail

version="${1:?usage: formula.sh VERSION SHA_MACOS_ARM SHA_MACOS_INTEL SHA_LINUX_ARM SHA_LINUX_INTEL}"
macos_arm="${2:?}"
macos_intel="${3:?}"
linux_arm="${4:?}"
linux_intel="${5:?}"
base="https://github.com/oval-vote/oval/releases/download/v$version/oval-$version"

cat <<FORMULA
class Oval < Formula
  desc "Command-line tools for the Oval voting guide format"
  homepage "https://oval.vote"
  license "MIT"

  on_macos do
    on_arm do
      url "$base-aarch64-apple-darwin.tar.gz"
      sha256 "$macos_arm"
    end
    on_intel do
      url "$base-x86_64-apple-darwin.tar.gz"
      sha256 "$macos_intel"
    end
  end

  on_linux do
    on_arm do
      url "$base-aarch64-unknown-linux-musl.tar.gz"
      sha256 "$linux_arm"
    end
    on_intel do
      url "$base-x86_64-unknown-linux-musl.tar.gz"
      sha256 "$linux_intel"
    end
  end

  def install
    bin.install "oval"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/oval --version")
    (testpath/"guide.oval").write <<~OVAL
      oval 1
      guide "G"
      election "E"
      date 2026-11-03
      ballot @b "B"
        measure @m "M"
          recommend yes
    OVAL
    system bin/"oval", "check", testpath/"guide.oval"
  end
end
FORMULA
