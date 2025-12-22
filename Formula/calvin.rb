# typed: false
# frozen_string_literal: true

class Calvin < Formula
  desc "PromptOps compiler - write once, compile to all AI coding assistants"
  homepage "https://github.com/64andrewwalker/calvin"
  version "0.3.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/64andrewwalker/calvin/releases/download/v#{version}/calvin-aarch64-apple-darwin.tar.gz"
      sha256 "11506ead63d2b8945c0d8ccbd6974d2e950b5a6156b842c496da54381279e726"
    end
    on_intel do
      url "https://github.com/64andrewwalker/calvin/releases/download/v#{version}/calvin-x86_64-apple-darwin.tar.gz"
      sha256 "2640f23f5fe6057bd8e06a242e686693d1a9ad7360e818c5091afc06cc1b89f1"
    end
  end

  on_linux do
    on_arm do
      url "https://github.com/64andrewwalker/calvin/releases/download/v#{version}/calvin-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "1a20157eb0d432254ee6b865cb6ff1f8746f33b97fe43e198bddba90ea8c8b71"
    end
    on_intel do
      url "https://github.com/64andrewwalker/calvin/releases/download/v#{version}/calvin-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "48fd5700bbf1ebde8b6b5c846c93cf09df5780bcc42b691d3f046a7ab9c3e877"
    end
  end

  def install
    bin.install "calvin"
  end

  test do
    assert_match "calvin #{version}", shell_output("#{bin}/calvin --version")
  end
end
