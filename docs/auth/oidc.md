# OIDC (OpenID Connect) Integration

## P3-4: OIDC Integration for Enterprise SSO

This document describes the OIDC integration for EVIF enterprise customers.

## Overview

OIDC (OpenID Connect) allows enterprise customers to authenticate using their existing identity providers (IdP) such as:
- Okta
- Azure Active Directory (Azure AD)
- Google Workspace
- Auth0
- Keycloak
- Ping Identity

## Configuration

### Environment Variables

```bash
# OIDC Configuration
EVIF_AUTH_OIDC_ENABLED=true
EVIF_AUTH_OIDC_ISSUER=https://your-okta.com
EVIF_AUTH_OIDC_CLIENT_ID=${OKTA_CLIENT_ID}
EVIF_AUTH_OIDC_CLIENT_SECRET=${OKTA_CLIENT_SECRET}
EVIF_AUTH_OIDC_REDIRECT_URI=https://your-app.evif.io/auth/callback
EVIF_AUTH_OIDC_SCOPES=openid,profile,email,groups
EVIF_AUTH_OIDC_ROLE_MAPPING_ENABLED=true
```

### Configuration File (evif.toml)

```toml
[auth]
# OIDC Authentication
[auth.oidc]
enabled = true
issuer = "https://your-okta.com"
client_id = "${OKTA_CLIENT_ID}"
client_secret = "${OKTA_CLIENT_SECRET}"
redirect_uri = "https://your-app.evif.io/auth/callback"

# Scopes
scopes = ["openid", "profile", "email", "groups"]

# Role Mapping (IdP groups → EVIF roles)
[auth.oidc.role_mapping]
admin_group = "evif-admins"
user_group = "evif-users"
readonly_group = "evif-readonly"

# JIT (Just-In-Time) Provisioning
jit_provisioning = true

# Session Configuration
[auth.session]
max_age_seconds = 86400  # 24 hours
idle_timeout_seconds = 3600  # 1 hour
absolute_timeout_seconds = 604800  # 7 days

# MFA Requirement
require_mfa = false  # Set to true for enterprise compliance
```

## Supported Identity Providers

### Okta Configuration

```toml
[auth.oidc]
issuer = "https://your-org.okta.com"
client_id = "${OKTA_CLIENT_ID}"
client_secret = "${OKTA_CLIENT_SECRET}"

# Okta-specific settings
[auth.oidc.okta]
domain = "your-org.okta.com"
```

**Okta Setup Steps:**
1. Create an OIDC application in Okta Admin
2. Set the redirect URI to `https://your-app.evif.io/auth/callback`
3. Assign groups: `evif-admins`, `evif-users`
4. Configure claim mappings for role extraction

### Azure AD Configuration

```toml
[auth.oidc]
issuer = "https://login.microsoftonline.com/{tenant}/v2.0"
client_id = "${AZURE_CLIENT_ID}"
client_secret = "${AZURE_CLIENT_SECRET}"

# Azure AD-specific settings
[auth.oidc.azure]
tenant_id = "${AZURE_TENANT_ID}"
```

**Azure AD Setup Steps:**
1. Register an application in Azure AD
2. Configure the redirect URI
3. Expose API permissions for Graph API (for group claims)
4. Map application roles to EVIF roles

### Google Workspace Configuration

```toml
[auth.oidc]
issuer = "https://accounts.google.com"
client_id = "${GOOGLE_CLIENT_ID}"
client_secret = "${GOOGLE_CLIENT_SECRET}"
scopes = ["openid", "profile", "email"]

# Google-specific
[auth.oidc.google]
hd = "your-domain.com"  # Restrict to specific domain
```

## Role Mapping

### How It Works

```
IdP Group          →  EVIF Role
─────────────────────────────────
evif-admins        →  admin
evif-users         →  member
evif-readonly     →  readonly
```

### Custom Role Mapping Example

```toml
[auth.oidc.role_mapping]
# Map Okta groups to EVIF roles
"CN=EVIF Admins,OU=Groups,DC=company,DC=com" = "admin"
"CN=EVIF Users,OU=Groups,DC=company,DC=com" = "member"
"CN=EVIF ReadOnly,OU=Groups,DC=company,DC=com" = "readonly"
```

## User Provisioning (JIT)

### Just-In-Time Provisioning Flow

```
1. User visits EVIF → Redirect to IdP
2. User authenticates with IdP
3. IdP returns ID token with claims
4. EVIF extracts user info + groups
5. EVIF creates/updates user if not exists
6. EVIF assigns roles based on group mapping
7. User is logged in
```

### User Attributes Mapping

| IdP Claim | EVIF Field |
|-----------|-----------|
| `sub` | `external_id` |
| `email` | `email` |
| `name` or `preferred_username` | `display_name` |
| Groups claim | `roles` |

## Session Management

### Token Storage

JWT tokens are stored in HTTP-only cookies for security:

```
Set-Cookie: evif_session=<jwt_token>; HttpOnly; Secure; SameSite=Lax; Path=/
```

### Token Validation

```rust
// Token validation flow
async fn validate_token(token: &str) -> Result<UserClaims> {
    // 1. Verify signature with IdP's public keys (JWKS)
    // 2. Verify issuer matches configured issuer
    // 3. Verify audience matches our client_id
    // 4. Check token not expired
    // 5. Extract user claims
}
```

### Session Timeout

| Setting | Default | Description |
|---------|---------|-------------|
| `max_age_seconds` | 86400 | Maximum session duration |
| `idle_timeout_seconds` | 3600 | Session timeout if inactive |
| `absolute_timeout_seconds` | 604800 | Absolute timeout (7 days) |

## MFA (Multi-Factor Authentication)

### Enforcing MFA

```toml
[auth.mfa]
required_for_roles = ["admin", "member"]  # Roles that must use MFA
allowed_methods = ["totp", "webauthn", "sms"]  # Available MFA methods
```

### MFA Flow

```
1. User logs in with password
2. EVIF checks if MFA required for user role
3. If required → redirect to MFA verification
4. User completes MFA challenge
5. Session is created with MFA verified flag
```

## SAML Support

For enterprise customers that require SAML instead of OIDC:

```toml
[auth.saml]
enabled = true
idp_metadata_url = "https://your-idp.com/metadata.xml"
sp_entity_id = "https://your-app.evif.io"
acs_url = "https://your-app.evif.io/auth/saml/acs"

# Certificate paths
idp_cert_path = "/etc/evif/idp-cert.pem"
sp_cert_path = "/etc/evif/sp-cert.pem"
sp_key_path = "/etc/evif/sp-key.pem"
```

## Troubleshooting

### Common Issues

**Token validation fails:**
```
Error: "invalid_token: JWT signature verification failed"
Solution: Check that JWKS endpoint is accessible from EVIF
```

**Groups not being mapped:**
```
Error: "user has no roles assigned"
Solution: Verify group claim is included in token and role mapping is configured
```

**Redirect URI mismatch:**
```
Error: "redirect_uri_mismatch"
Solution: Ensure redirect URI exactly matches configured in IdP
```

### Debug Mode

Enable debug logging for OIDC:

```bash
RUST_LOG=evif_auth=debug,oidc=debug cargo run
```

### Testing OIDC Integration

```bash
# Test with a specific issuer
curl -X POST https://your-app.evif.io/auth/oidc/test \
  -H "Content-Type: application/json" \
  -d '{"issuer": "https://your-okta.com"}'
```

## Security Considerations

- All tokens are validated on every request
- Tokens are never stored in localStorage (XSS protection)
- Refresh tokens rotate on each use
- Session cookies are HttpOnly, Secure, SameSite=Lax
- IdP public keys (JWKS) are cached and refreshed periodically

## API Reference

### Authentication Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/auth/login` | GET | Initiate OIDC login flow |
| `/auth/callback` | GET | OIDC callback handler |
| `/auth/logout` | POST | Logout and invalidate session |
| `/auth/refresh` | POST | Refresh access token |
| `/auth/userinfo` | GET | Get current user info |

### Admin Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/admin/sso/config` | GET/PUT | SSO configuration |
| `/admin/sso/test` | POST | Test SSO connection |
| `/admin/users` | GET | List users with SSO status |
