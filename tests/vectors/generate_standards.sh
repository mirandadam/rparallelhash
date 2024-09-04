# Generate standard checksums for the test vectors using the linux command line.
# Non-standard requirements: b3sum, openssl
md5sum *.bin > md5.txt
sha1sum *.bin > sha1.txt
sha256sum *.bin > sha256.txt
sha384sum *.bin > sha384.txt
sha512sum *.bin > sha512.txt
openssl dgst -r -sha3-256 *.bin | sed 's/ \*/  /' > sha3-256.txt
openssl dgst -r -sha3-384 *.bin | sed 's/ \*/  /' > sha3-384.txt
openssl dgst -r -sha3-512 *.bin | sed 's/ \*/  /' > sha3-512.txt
b3sum *.bin > blake3.txt
