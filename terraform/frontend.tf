resource "aws_s3_bucket" "prod" {
  bucket = "inband-traceroute-prod"
}


resource "aws_s3_bucket_cors_configuration" "default" {
  bucket   = aws_s3_bucket.prod.id

  cors_rule {
    allowed_methods = ["GET"]
    allowed_origins = ["*"]
  }
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

import {
  to = cloudflare_ruleset.http_request_transform
  id = "zones/03677931e8b59074e965e3b8a7351b54/d12a8c5c5c6c475e9631da99f587fc19"
}

resource "cloudflare_ruleset" "http_request_dynamic_redirect" {
  zone_id     = cloudflare_zone.inband_traceroute.id
  name        = "default"
  description = "HTTP request dynamic redirect ruleset"
  kind        = "zone"
  phase       = "http_request_dynamic_redirect"

  rules = [{
    action = "redirect"
    action_parameters = {
      from_value = {
        preserve_query_string = true
        status_code           = 301
        target_url = {
          expression = "concat(\"https://\", http.host, http.request.uri.path)"
        }
      }
    }
    description = "HTTP to HTTPs"
    enabled     = true
    expression  = "(not ssl) and not starts_with(http.request.uri.path, \"/.well-known/\")"
  }]
}
import {
  to = cloudflare_ruleset.http_request_dynamic_redirect
  id = "zones/03677931e8b59074e965e3b8a7351b54/bf28f0f018bc4a4cbabd0db6446f342b"
}
