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
    scaleway = {
      source  = "scaleway/scaleway"
      version = "2.53.0"
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
