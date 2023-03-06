#!/bin/bash

openssl genrsa -out ca.key 2048
openssl req -new -subj "/C=BR/ST=Parana/L=Pato Branco/O=UTFPR/OU=PB/CN=UTFPR-PB-RSA-2048" -x509 -sha256 -days 3650 -extensions v3_req -config <(cat /etc/ssl/openssl.cnf <(printf "\n[v3_req]\nbasicConstraints=critical,CA:TRUE\nkeyUsage=nonRepudiation,digitalSignature,keyEncipherment\nsubjectAltName=DNS:localhost")) -key ca.key -out ca.pem
# Create Truststore adding a CA
keytool -keystore truststore.p12 -storepass password -alias CARoot -import -file ca.pem

openssl genrsa -out server.key 2048
openssl req -new -subj "/C=BR/ST=Parana/L=Pato Branco/O=UTFPR/OU=PB/CN=UTFPR-PB-server-RSA-2048" -addext "subjectAltName = DNS:localhost" -key server.key -out server.csr
openssl x509 -req -sha256 -days 3650 -extfile <(printf "subjectAltName=DNS:localhost") -in server.csr -CA ca.pem -CAkey ca.key -set_serial 01 -out server.pem

openssl genrsa -out client.key 2048
openssl req -new -subj "/C=BR/ST=Parana/L=Pato Branco/O=UTFPR/OU=PB/CN=UTFPR-PB-client01-RSA-2048" -key client.key -out client.csr
openssl x509 -req -sha256 -days 3650 -extfile <(printf "subjectAltName=DNS:localhost") -in client.csr -CA ca.pem -CAkey ca.key -set_serial 02 -out client.pem
