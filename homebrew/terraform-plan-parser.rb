class TerraformPlanParser < Formula
  desc "Lightweight CLI that turns Terraform plan JSON into clean summaries"
  homepage "https://github.com/billybox1926-jpg/terraform-plan-parser"
  version "0.1.0"

  on_macos do
    if Hardware::CPU.intel?
      url "https://github.com/billybox1926-jpg/terraform-plan-parser/releases/download/v0.1.0/terraform_plan_parser-macos-x64.tar.gz"
      sha256 "e4e429677d41d69db1b644a5cf5fc2238aa98305c7b65ef498f2ebcac8ed6376"
    end
  end

  on_linux do
    if Hardware::CPU.intel?
      url "https://github.com/billybox1926-jpg/terraform-plan-parser/releases/download/v0.1.0/terraform_plan_parser-linux-x64.tar.gz"
      sha256 "904554101fd540fdecb5d41e37a8a49b61fad62b5b93675578d88af2293e30d0"
    end
  end

  def install
    bin.install "terraform_plan_parser"
  end

  test do
    assert_match "terraform_plan_parser", shell_output("#{bin}/terraform_plan_parser --version")
  end
end
