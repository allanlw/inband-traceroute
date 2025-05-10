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
    cloudflare = {
      source  = "cloudflare/cloudflare"
      version = "~> 5"
    }
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5"
    }
    namecheap = {
      source  = "namecheap/namecheap"
      version = ">= 2.0.0"
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

provider "cloudflare" {
}

provider "cloudflare" {
  alias     = "r2"
  api_token = var.cloudflare_r2_api_key
}

provider "aws" {
  region = "us-east-1"

  access_key = var.cloudflare_access_key_id
  secret_key = var.cloudflare_secret_access_key

  # Required for R2.
  # These options disable S3-specific validation on the client (Terraform) side.
  skip_credentials_validation = true
  skip_region_validation      = true
  skip_requesting_account_id  = true

  endpoints {
    s3 = "https://${var.cloudflare_account_id}.r2.cloudflarestorage.com"
  }
}

provider "namecheap" {}
