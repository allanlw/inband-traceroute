locals {
  domain_name   = "inband-traceroute.net"
  vultr_regions = toset(["nrt", "lhr", /* "atl",  "syd", "sao", "jnb" */])
}
