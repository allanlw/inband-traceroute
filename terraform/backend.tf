terraform {
  backend "gcs" {
    bucket  = "inband-traceroute-terraform-state"
    prefix  = "terraform/state"
  }
}
