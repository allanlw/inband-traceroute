provider "google" {
  project = "inband-traceroute"
}

provider "vultr" {
  api_key = var.vultr_api_key
}

provider "scaleway" {
  project_id = "716a49bf-33f5-4793-ac51-3a45fdc0341b" # inband-traceroute
  region     = "fr-par"
  zone       = "fr-par-2"
}

provider "github" {}
