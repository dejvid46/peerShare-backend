#!/bin/bash

openssl req -x509 -newkey rsa:4096 -sha256 -days 3650 \
  -nodes -keyout server.key -out server.crt -subj "/CN=peer-share.net" \
  -addext "subjectAltName=DNS:peer-share.net,DNS:*.peer-share.net,IP:146.190.178.221"