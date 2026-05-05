# typed: strict
# frozen_string_literal: true

# EVIF - Everything Is a File
# A persistent context, skills, and multi-agent coordination system for AI agents.
#
# Homepage: https://evif.io
# GitHub:   https://github.com/evif/evif

require "uri"
require "json"

class Evif < Formula
  desc "EVIF - Everything Is a File for AI Agents"
  homepage "https://evif.io"
  license "MIT OR Apache-2.0"
  url "https://github.com/evif/evif/releases/download/v0.2.0/evif-#{`uname -s`.downcase}-#{`uname -m`.strip}.tar.gz"
  version "0.2.0"

  # Universal binary support
  if Hardware::CPU.intel? && Hardware::CPU.is_64_bit?
    url "https://github.com/evif/evif/releases/download/v0.2.0/evif-x86_64-apple-darwin.tar.gz"
  elsif Hardware::CPU.arm?
    url "https://github.com/evif/evif/releases/download/v0.2.0/evif-aarch64-apple-darwin.tar.gz"
  end

  head "https://github.com/evif/evif.git", branch: "main"

  depends_on "openssl@3"

  def install
    # Install binaries
    bin.install "evif" => "evif"

    # Create directories for EVIF data
    (prefix/"evif").mkpath
    (prefix/"evif/skills").mkpath
    (prefix/"evif/config").mkpath
    (prefix/"evif/cache").mkpath
    (prefix/"evif/logs").mkpath

    # Install man page if available
    man1.install "evif.1" if File.exist?("evif.1")

    # Create bash/zsh completions
    (prefix/"completions").mkpath
    generate_completions!

    # Install example config
    etc.install "evif.toml.example" if File.exist?("evif.toml.example")
  end

  def caveats
    <<~EOS
      EVIF has been installed!

      Quick start:
        evif --help              # Show help
        evif connect claude      # Connect to Claude Desktop
        evif skill ls            # List available skills
        evif health              # Check server status

      Configuration:
        ~/.evif/                 # EVIF data directory
        ~/.evif/config/          # Configuration files
        ~/.evif/skills/          # Custom skills

      For MCP Server:
        evif mcp serve          # Start MCP server on stdio

      Documentation: https://evif.io/docs
    EOS
  end

  def generate_completions!
    # Generate shell completions using evif built-in
    return unless which("evif")

    completions = {
      "bash" => "#{prefix}/completions/evif.bash",
      "zsh"  => "#{prefix}/completions/_evif",
      "fish" => "#{prefix}/completions/evif.fish"
    }

    completions.each do |shell, path|
      Utils.safe_system "evif", "completion", "--shell", shell, ">", path
    rescue
      # Ignore completion generation errors
    end
  end

  test do
    # Basic CLI test
    assert_match "evif", shell_output("#{bin}/evif --version")

    # Help works
    assert_match "Usage:", shell_output("#{bin}/evif --help")

    # Connect command exists
    assert_match "connect", shell_output("#{bin}/evif connect --help")
  end
end