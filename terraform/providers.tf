terraform {
  required_providers {
    google = {
      source  = "hashicorp/google"
      version = "6.30.0"
    }
    vultr = {
      source  = "vultr/vultr"
      version = "2.26.0"
    }
    github = {
      source  = "integrations/github"
      version = "6.6.0"
    }
    tls = {
      source  = "hashicorp/tls"
      version = "4.1.0"
    }
  }
}

provider "google" {
  project = "inband-traceroute"
}

provider "vultr" {
  api_key = var.vultr_api_key
}

provider "github" {}
