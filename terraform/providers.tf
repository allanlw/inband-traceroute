provider "google" {
  project = "inband-traceroute"
}

provider "vultr" {
  api_key = var.vultr_api_key
}

provider "github" {}
