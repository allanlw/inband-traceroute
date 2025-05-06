resource "vultr_ssh_key" "inband_traceroute_tf" {
  name    = "inband-traceroute-tf"
  ssh_key = var.ssh_pubkey
}

resource "vultr_startup_script" "init" {
  name   = "inband-traceroute"
  script = base64encode(terraform_data.cloud_init.output)
}

resource "vultr_instance" "dev" {
  count = local.dev_mode == "vultr" ? 1 : 0

  label    = "inband-traceroute-dev"
  hostname = "inband-traceroute-dev"

  region = "nrt"
  plan   = "vc2-4c-8gb"

  os_id       = 2136 # Debian 12 x64
  enable_ipv6 = true
  user_scheme = "limited"
  ssh_key_ids = [
    vultr_ssh_key.inband_traceroute_tf.id
  ]
  script_id = vultr_startup_script.init.id

  lifecycle {
    replace_triggered_by = [
      vultr_startup_script.init.script,
    ]
  }
}
