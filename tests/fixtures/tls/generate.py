"""Gera certificados sintéticos locais; nenhuma identidade real é utilizada.

Requer Python com cryptography. Executar a partir de qualquer diretório.
A senha pública 'test-only' serve apenas para exercitar o parser de PKCS#12.
As chaves não têm uso fora destes testes. Não se grava a chave da CA em disco.
"""

from datetime import datetime, timezone
from pathlib import Path

from cryptography import x509
from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import rsa
from cryptography.hazmat.primitives.serialization import pkcs12
from cryptography.x509.oid import ExtendedKeyUsageOID, NameOID

OUT = Path(__file__).parent
START = datetime(2020, 1, 1, tzinfo=timezone.utc)
END = datetime(2045, 1, 1, tzinfo=timezone.utc)


def key():
    return rsa.generate_private_key(public_exponent=65537, key_size=2048)


def certificate(name, private_key, issuer=None, issuer_key=None, ca=False,
                purpose=None, expired=False):
    subject = x509.Name([x509.NameAttribute(NameOID.COMMON_NAME, name)])
    builder = (x509.CertificateBuilder()
               .subject_name(subject)
               .issuer_name(issuer.subject if issuer else subject)
               .public_key(private_key.public_key())
               .serial_number(x509.random_serial_number())
               .not_valid_before(START)
               .not_valid_after(datetime(2021, 1, 1, tzinfo=timezone.utc) if expired else END)
               .add_extension(x509.BasicConstraints(ca=ca, path_length=None), critical=True)
               .add_extension(x509.KeyUsage(
                   digital_signature=True, content_commitment=False,
                   key_encipherment=not ca, data_encipherment=False,
                   key_agreement=False, key_cert_sign=ca, crl_sign=ca,
                   encipher_only=False, decipher_only=False), critical=True)
               .add_extension(x509.SubjectKeyIdentifier.from_public_key(private_key.public_key()),
                              critical=False)
               .add_extension(x509.AuthorityKeyIdentifier.from_issuer_public_key(
                   (issuer_key or private_key).public_key()), critical=False))
    if purpose:
        builder = builder.add_extension(x509.ExtendedKeyUsage([purpose]), critical=False)
    if purpose == ExtendedKeyUsageOID.SERVER_AUTH:
        builder = builder.add_extension(
            x509.SubjectAlternativeName([x509.DNSName("localhost")]), critical=False)
    return builder.sign(issuer_key or private_key, hashes.SHA256())


root_key = key()
root = certificate("fiscal-rs test root", root_key, ca=True)
intermediate_key = key()
intermediate = certificate("fiscal-rs test intermediate", intermediate_key,
                           root, root_key, ca=True)
client_key = key()
client = certificate("fiscal-rs test client", client_key, intermediate,
                     intermediate_key, purpose=ExtendedKeyUsageOID.CLIENT_AUTH)
server_key = key()
server = certificate("localhost", server_key, root, root_key,
                     purpose=ExtendedKeyUsageOID.SERVER_AUTH)
expired = certificate("localhost expired", server_key, root, root_key,
                      purpose=ExtendedKeyUsageOID.SERVER_AUTH, expired=True)
for name, cert in [("root.pem", root), ("client.pem", client),
                   ("intermediate.pem", intermediate), ("server.pem", server),
                   ("expired-server.pem", expired)]:
    (OUT / name).write_bytes(cert.public_bytes(serialization.Encoding.PEM))
(OUT / "server-key.pem").write_bytes(server_key.private_bytes(
    serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8,
    serialization.NoEncryption()))
for name, chain in [("client-with-chain.pfx", [root, intermediate]),
                    ("client-without-chain.pfx", None)]:
    (OUT / name).write_bytes(pkcs12.serialize_key_and_certificates(
        b"synthetic-test-client", client_key, client, chain,
        serialization.BestAvailableEncryption(b"test-only")))
