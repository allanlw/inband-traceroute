resource "tls_private_key" "inband_traceroute_deploy_key" {
  algorithm = "ED25519"
}

resource "github_repository_deploy_key" "inband_traceroute_deploy_key" {
  title      = "Terraform managed deploy key"
  repository = "inband-traceroute"
  key        = tls_private_key.inband_traceroute_deploy_key.public_key_openssh
  read_only  = false
}
