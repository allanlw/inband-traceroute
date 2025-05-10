resource "aws_s3_bucket" "prod" {
  bucket = "inband-traceroute-prod"
}

resource "aws_s3_object" "index" {
  bucket       = aws_s3_bucket.prod.id
  key          = "index.html"
  content_type = "text/html"
  source       = "${path.module}/../frontend/index.html"
  lifecycle {
    ignore_changes = [content_encoding]
  }
}


resource "aws_s3_object" "nodes" {
  bucket       = aws_s3_bucket.prod.id
  key          = "nodes.json"
  content_type = "text/json"
  content      = jsonencode(module.trace_node)

  lifecycle {
    ignore_changes = [content_encoding]
  }
}


# https://github.com/cloudflare/terraform-provider-cloudflare/issues/5567
resource "cloudflare_r2_custom_domain" "frontend" {
  provider    = cloudflare.r2
  account_id  = var.cloudflare_account_id
  bucket_name = aws_s3_bucket.prod.bucket
  domain      = local.domain_name
  enabled     = true
  zone_id     = cloudflare_zone.inband_traceroute.id
  min_tls     = "1.2"
  depends_on  = [aws_s3_bucket.prod]
}

// terraform import cloudflare_ruleset.http_request_transform 'zones/03677931e8b59074e965e3b8a7351b54/d12a8c5c5c6c475e9631da99f587fc19'
resource "cloudflare_ruleset" "http_request_transform" {
  zone_id     = cloudflare_zone.inband_traceroute.id
  name        = "default"
  description = "HTTP request transform ruleset"
  kind        = "zone"
  phase       = "http_request_transform"

  rules = [{
    ref         = "rewrite_index"
    description = "Rewrite / to /index.html"
    expression  = "(http.host eq \"${local.domain_name}\" and http.request.uri.path eq \"/\")"
    action      = "rewrite"
    action_parameters = {
      uri = {
        path = {
          value = "/index.html"
        }
      }
    }
  }]
}
