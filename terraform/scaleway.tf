resource "scaleway_instance_ip" "dev_ipv4" {
  count = local.dev_mode == "scaleway" ? 1 : 0
  type  = "routed_ipv4"
}

resource "scaleway_instance_ip" "dev_ipv6" {
  count = local.dev_mode == "scaleway" ? 1 : 0
  type  = "routed_ipv6"
}


resource "scaleway_instance_server" "dev" {
  count = local.dev_mode == "scaleway" ? 1 : 0
  type  = "DEV1-L" # -S, -M or -L
  name  = "inband-traceroute-dev"
  image = "debian_bookworm"

  root_volume {
    size_in_gb            = 64
    delete_on_termination = true
  }

  ip_ids = [
    scaleway_instance_ip.dev_ipv4[0].id,
    scaleway_instance_ip.dev_ipv6[0].id
  ]

  user_data = {
    cloud-init = terraform_data.cloud_init.output
  }

  lifecycle {
    replace_triggered_by = [
      terraform_data.cloud_init,
    ]
  }
}
