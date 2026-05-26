#!/bin/sh
set -eu

# Render the nginx config from the template, injecting the listen port and the
# backend upstream. Only these two vars are substituted so nginx's own $-vars
# in the template are left untouched.
: "${PORT:=80}"
: "${BACKEND_URL:=http://backend:8787}"

export PORT BACKEND_URL
envsubst '${PORT} ${BACKEND_URL}' \
    < /etc/nginx/templates/default.conf.template \
    > /etc/nginx/conf.d/default.conf

echo "frontend: listening on :${PORT}, proxying /api -> ${BACKEND_URL}"
exec nginx -g 'daemon off;'
