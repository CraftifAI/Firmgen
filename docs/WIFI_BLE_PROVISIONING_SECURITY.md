# Security Model for WiFi Provisioning over BLE

Concise design for embedded engineers: threat model, layered security, protocols/parameters, fallback, and secure credential erasure.

---

## 1. Threat Model

### 1.1 Attacker Capabilities (during provisioning)

| Capability | Description |
|------------|-------------|
| **Passive eavesdropping** | Sniff BLE link; read GATT traffic (credentials, SSID, passphrase). |
| **Active MITM** | Relay or modify messages between phone and device; inject/modify provisioning frames. |
| **Replay** | Capture provisioning session and replay later to re-provision or fingerprint. |
| **Physical/local access** | Read flash (NVS), debug interface, or extract keys if stored insecurely. |
| **Malicious app** | Compromised or fake provisioning app exfiltrating credentials. |

### 1.2 Risks

- **Credential theft**: WiFi SSID/passphrase captured → network access.
- **Device takeover**: Attacker provisions device to hostile AP or changes config.
- **Permanent secret exposure**: Credentials or session keys stored in plaintext in NVS.
- **Weakening on reset**: Credentials not securely erased → recoverable from flash.

### 1.3 Trust Assumptions

- User runs a legitimate provisioning app (or we minimize damage via device-side checks).
- Device and phone can perform ECDH and AES (or we have a defined fallback).
- Secure erasure is possible (e.g. overwrite NVS or use flash encrypt).

---

## 2. Recommended Security Measures

### 2.1 BLE-Level

| Measure | Recommendation |
|---------|----------------|
| **Pairing/bonding** | Use BLE pairing so link is encrypted. Prefer **bonding** so LTK is stored and subsequent connections are secure without re-pairing. |
| **LESC (Secure Connections)** | Prefer **LE Secure Connections** (BLE 4.2+): ECDH-based, no passkey in clear, resistant to passive eavesdropping and many MITM scenarios when combined with proper IO capability. |
| **MITM protection** | Use **MITM protection** (e.g. `ESP_LE_AUTH_REQ_SC_MITM` or equivalent): require Numeric Comparison or Passkey Entry so an attacker cannot pair without user action. |
| **IO capability** | Device: **DisplayOnly** or **NoInputNoOutput**; app: **KeyboardDisplay** or **KeyboardOnly**. Avoid **NoInputNoOutput** on both sides (no MITM protection). |
| **Key size** | Request maximum encryption key size (e.g. **16 bytes**). |

**Fallback when LESC not available**: See §5 (backward compatibility).

### 2.2 Transport-Level (application protocol over BLE)

- **Ephemeral ECDH**: Run an ECDH key agreement over the (optionally) BLE-encrypted link to get a shared secret; derive a **session encryption key** with HKDF. This gives **forward secrecy** per session and protects even if BLE keys are later compromised.
- **Authenticated encryption**: Encrypt and authenticate all credential-bearing messages with an AEAD (e.g. **AES-128-GCM** or **AES-256-GCM**).
- **Binding to BLE link**: Include BLE connection identifiers or session nonces in HKDF so keys are not reusable across connections/sessions.

### 2.3 Storage-Level

- **At rest**: Never store WiFi credentials in NVS in plaintext.
  - **Option A**: Encrypt credentials in NVS with a key from **HKDF(device_unique_secret, "nvs-wifi-key", …)**. Device_unique_secret can be from efuse, or from a **secure element / TPM** if available.
  - **Option B**: Use a **secure element** or **TPM** to store or wrap the encryption key; device firmware only gets a wrapped key or session-bound decryption.
- **Integrity**: Use AEAD (e.g. AES-GCM) for NVS credential blobs so tampering is detected.

---

## 3. Protocols and Parameters

### 3.1 Standard Choices

| Item | Value | Notes |
|------|--------|------|
| **Curve** | **P-256** (NIST, or **secp256r1**) | BLE LESC uses P-256; reuse for app-layer ECDH. |
| **ECDH** | Ephemeral key per session | Generate new key pair each provisioning session. |
| **Key derivation** | **HKDF-SHA256** | IANA-style: HKDF-Extract then HKDF-Expand. |
| **AEAD (transport)** | **AES-128-GCM** or **AES-256-GCM** | 128-bit or 256-bit key; 96-bit nonce (random or counter). |
| **AEAD (NVS)** | **AES-128-GCM** or **AES-256-GCM** | Same; IV/nonce per blob, never reuse. |
| **Key lengths** | 128 or 256 bit | Match AEAD (128-bit minimum). |

### 3.2 ECDH + HKDF → Session Key (pseudocode)

```c
// --- Constants ---
#define ECDH_PRIVATE_KEY_BYTES  32   // P-256
#define ECDH_PUBLIC_KEY_BYTES   64   // uncompressed: 32 X + 32 Y
#define SHARED_SECRET_BYTES     32   // P-256 shared secret
#define HKDF_KEY_LEN            16   // 128-bit for AES-128-GCM
#define HKDF_NONCE_LEN          12   // 96-bit for GCM
#define HKDF_INFO_TRANSPORT     "wifi-provisioning-v1-transport"

// --- Device side: key exchange and derive keys ---
void device_provisioning_crypto(void) {
    uint8_t dev_priv[ECDH_PRIVATE_KEY_BYTES];
    uint8_t dev_pub[ECDH_PUBLIC_KEY_BYTES];
    uint8_t peer_pub[ECDH_PUBLIC_KEY_BYTES];   // from phone
    uint8_t shared[SHARED_SECRET_BYTES];
    uint8_t session_key[HKDF_KEY_LEN];
    uint8_t session_nonce[HKDF_NONCE_LEN];

    // 1) Generate ephemeral key pair (P-256)
    ecdh_generate_keypair(P256, dev_priv, dev_pub);  // e.g. mbedtls_ecdh_*

    // 2) Send dev_pub to phone over BLE; receive peer_pub
    ble_send(dev_pub, sizeof(dev_pub));
    ble_recv(peer_pub, sizeof(peer_pub));

    // 3) ECDH: shared = ECDH(dev_priv, peer_pub)
    ecdh_compute_shared(P256, dev_priv, peer_pub, shared, sizeof(shared));

    // 4) Optional: mix in session/connection binding (e.g. BLE conn handle or nonces)
    uint8_t session_id[32];
    get_session_binding(session_id, sizeof(session_id));  // e.g. nonce_dev || nonce_phone

    // 5) HKDF: session_key = HKDF-SHA256(shared, session_id, "wifi-provisioning-v1-transport", 16)
    uint8_t prk[32];
    hkdf_extract(SHA256, shared, sizeof(shared), session_id, sizeof(session_id), prk);
    hkdf_expand(SHA256, prk, sizeof(prk), (uint8_t*)HKDF_INFO_TRANSPORT,
                strlen(HKDF_INFO_TRANSPORT), session_key, HKDF_KEY_LEN);

    // 6) Derive nonce/IV for AEAD (e.g. first 12 bytes of HKDF expand with different info)
    hkdf_expand(SHA256, prk, sizeof(prk), (uint8_t*)"wifi-provisioning-v1-nonce", 26,
                session_nonce, HKDF_NONCE_LEN);

    // 7) Erase sensitive material
    secure_zero(dev_priv, sizeof(dev_priv));
    secure_zero(shared, sizeof(shared));
    secure_zero(prk, sizeof(prk));

    // 8) Use session_key + session_nonce for AES-GCM encrypt/decrypt of credentials
    //    For multiple frames: use session_nonce as base and increment, or derive per-frame nonce.
}
```

### 3.3 Encrypt credentials before sending (pseudocode)

```c
#define GCM_TAG_LEN 16

void phone_encrypt_and_send_credentials(const uint8_t *ssid, size_t ssid_len,
                                        const uint8_t *passphrase, size_t pass_len,
                                        const uint8_t *session_key, const uint8_t *nonce) {
    // Build plaintext (use a small TLV or fixed format)
    size_t plain_len = 2 + ssid_len + 2 + pass_len;
    uint8_t *plain = heap_alloc(plain_len);
    plain[0] = (uint8_t)(ssid_len >> 8); plain[1] = (uint8_t)ssid_len;
    memcpy(plain + 2, ssid, ssid_len);
    plain[2+ssid_len]   = (uint8_t)(pass_len >> 8);
    plain[3+ssid_len]   = (uint8_t)pass_len;
    memcpy(plain + 4 + ssid_len, passphrase, pass_len);

    uint8_t cipher[plain_len + GCM_TAG_LEN];
    size_t cipher_len;

    aes_gcm_encrypt(session_key, 16, nonce, 12,
                    NULL, 0,           // no AAD, or add version/type
                    plain, plain_len,
                    cipher, &cipher_len,
                    cipher + plain_len, GCM_TAG_LEN);

    ble_send(cipher, cipher_len);
    secure_zero(plain, plain_len);
    free(plain);
}

void device_decrypt_and_store_credentials(const uint8_t *cipher, size_t cipher_len,
                                          const uint8_t *session_key, const uint8_t *nonce) {
    size_t plain_len = cipher_len - GCM_TAG_LEN;
    uint8_t *plain = heap_alloc(plain_len);
    int ret = aes_gcm_decrypt(session_key, 16, nonce, 12,
                              NULL, 0,
                              cipher, cipher_len - GCM_TAG_LEN,
                              cipher + plain_len, GCM_TAG_LEN,
                              plain);
    if (ret != 0) { /* abort: auth failed */ free(plain); return; }

    size_t ssid_len = (plain[0] << 8) | plain[1];
    size_t pass_len = (plain[2+ssid_len] << 8) | plain[3+ssid_len];
    // Validate: 4 + ssid_len + pass_len == plain_len
    store_wifi_credentials_encrypted(plain + 2, ssid_len, plain + 4 + ssid_len, pass_len);
    secure_zero(plain, plain_len);
    free(plain);
}
```

### 3.4 NVS storage (encrypted blob)

```c
#define NVS_KEY_LABEL "nvs-wifi-key"
#define NVS_CREDENTIAL_NAMESPACE "wifi_enc"

void store_wifi_credentials_encrypted(const uint8_t *ssid, size_t ssid_len,
                                      const uint8_t *pass, size_t pass_len) {
    uint8_t key[32];
    uint8_t nonce[12];
    uint8_t blob[256], tag[16];  // size for your max credential length

    // Derive NVS encryption key from device secret (efuse or SE/TPM)
    get_device_secret(key, sizeof(key));  // 32 bytes from efuse/SE
    hkdf_expand(SHA256, key, 32, (uint8_t*)NVS_KEY_LABEL, strlen(NVS_KEY_LABEL), key, 32);

    random_bytes(nonce, 12);
    size_t plain_len = 2 + ssid_len + 2 + pass_len;
    uint8_t *plain = stack_alloc(plain_len);
    // ... build plain as above ...
    size_t enc_len;
    aes_gcm_encrypt(key, 32, nonce, 12, NULL, 0, plain, plain_len, blob, &enc_len, tag, 16);

    nvs_set_blob(NVS_CREDENTIAL_NAMESPACE, "cred", blob, enc_len);
    nvs_set_blob(NVS_CREDENTIAL_NAMESPACE, "tag", tag, 16);
    nvs_set_blob(NVS_CREDENTIAL_NAMESPACE, "nonce", nonce, 12);
    nvs_commit();
    secure_zero(plain, plain_len);
    secure_zero(key, sizeof(key));
}
```

---

## 4. Backward-Compatible Fallback (no LESC)

When the device or the mobile app does not support LESC (Secure Connections):

1. **Prefer LESC when both support it**: Negotiate Secure Connections first (e.g. only accept LESC if supported).
2. **Fallback to Legacy Pairing**:
   - Use **Passkey Entry** (device shows 6-digit code, user enters in app) to get MITM protection.
   - Avoid **Just Works** when transmitting credentials (no MITM protection).
3. **Application-layer ECDH remains**: Even with Legacy Pairing, run the same **ephemeral ECDH + HKDF + AES-GCM** over the link. Then:
   - Passive eavesdropping on the BLE link still sees only encrypted credential payloads (protected by session key).
   - MITM during pairing can still set up a key with the device; combining with **Passkey Entry** reduces risk.
4. **Capability negotiation**: In the first provisioning frame, exchange flags, e.g. `supports_lesc`, `supports_ecdh`. If `supports_ecdh` is true on both sides, always run ECDH + HKDF and encrypt credentials; if false, document that unencrypted credential transfer is insecure and avoid or deprecate.

**Example negotiation (first message after connection):**

```c
// First frame: capability
struct prov_caps {
    uint8_t version;      // 1
    uint8_t supports_lesc;
    uint8_t supports_ecdh;
    uint8_t reserved;
};
// If both support_ecdh: run ECDH then send encrypted credentials.
// If either does not: either abort or fall back to plaintext (insecure; log warning).
```

---

## 5. Securely Erase Credentials on Factory Reset

1. **Overwrite then delete**: Before erasing NVS entries or partitions, **overwrite** credential blobs and any key material with random data or zeros, then erase.
   - NVS: read blob, overwrite in RAM, write back (or write zeros) then `nvs_erase_*` and `nvs_commit`.
   - If credentials are in a dedicated partition, overwrite that partition (e.g. with `esp_partition_write`) then erase.
2. **Erase NVS namespace**: `nvs_flash_erase_partition("nvs")` or erase the NVS partition and re-init. If you only erase keys, prefer overwriting the credential items first.
3. **Secure zero in RAM**: When clearing keys or credentials, use a `secure_zero()` that is not optimized away (e.g. `mbedtls_platform_zeroize` or volatile write loop).
4. **Flash encryption (optional)**: If flash encryption is enabled, erasing the partition is still recommended; the key is in efuse and old ciphertext is effectively unrecoverable after erase.
5. **Sequence (pseudocode)**:

```c
void factory_reset_secure_erase(void) {
    uint8_t buf[256];
    size_t len = sizeof(buf);

    nvs_handle_t h;
    nvs_open(NVS_CREDENTIAL_NAMESPACE, NVS_READWRITE, &h);
    nvs_get_blob(h, "cred", buf, &len);
    memset(buf, 0, len);
    nvs_set_blob(h, "cred", buf, len);
    nvs_commit(h);
    nvs_erase_key(h, "cred");
    nvs_erase_key(h, "tag");
    nvs_erase_key(h, "nonce");
    nvs_commit(h);
    nvs_close(h);
    secure_zero(buf, sizeof(buf));

    // If full factory reset: erase entire NVS partition and re-init
    // nvs_flash_erase(); nvs_flash_init();
}
```

---

## 6. Summary Checklist for Implementers

- [ ] **BLE**: Enable pairing with MITM protection; prefer LESC + bonding; key size 16.
- [ ] **Transport**: Ephemeral ECDH (P-256) → HKDF-SHA256 → AES-128-GCM (or 256); encrypt all credential frames.
- [ ] **Storage**: Encrypt credentials in NVS (or SE/TPM) with AES-GCM; key from HKDF(device_secret, "nvs-wifi-key").
- [ ] **Fallback**: If no LESC, use Passkey Entry + same app-layer ECDH/AEAD; negotiate capabilities.
- [ ] **Reset**: Overwrite credential blobs and keys, then erase; use secure_zero for RAM; optionally full NVS erase.

Use standard libraries (e.g. mbedTLS, TinyCrypt) for ECDH P-256, HKDF-SHA256, and AES-GCM; avoid custom crypto.
