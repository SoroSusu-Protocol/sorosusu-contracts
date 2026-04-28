# SoroSusu Passkey Authentication Guide

## Overview

SoroSusu now supports **Stellar Protocol 21+ Passkeys** (WebAuthn) for biometric authentication, enabling users to sign transactions using FaceID, TouchID, or other biometric methods instead of managing 12-word recovery phrases. This represents a major leap in Web3 UX, making SoroSusu feel like a standard banking app while maintaining self-custody.

## What are Passkeys?

Passkeys are cryptographic credentials based on the **secp256r1 elliptic curve** that:
- Are generated and stored securely on user devices
- Use existing biometric systems (FaceID, TouchID, Windows Hello)
- Never expose private keys to applications or networks
- Provide phishing-resistant authentication
- Sync across devices via platform keychains

## Architecture

### Protocol Support
- **Stellar Protocol 21+**: Native secp256r1 signature verification
- **WebAuthn API**: Browser-based passkey generation and signing
- **Smart Wallet Pattern**: Passkeys as signers for programmable accounts

### Dual Authentication System
SoroSusu maintains backward compatibility while supporting passkeys:

1. **Traditional Ed25519** (default for existing users)
2. **Secp256r1 Passkeys** (new biometric option)

## Implementation Details

### Data Structures

#### PasskeyCredential
```rust
pub struct PasskeyCredential {
    pub public_key: BytesN<33>,        // Compressed secp256r1 public key
    pub credential_id: Bytes,          // WebAuthn credential identifier
    pub origin: Symbol,                 // Bound domain/origin
    pub registered_at: u64,            // Registration timestamp
    pub is_active: bool,               // Credential status
}
```

#### UserAuthProfile
```rust
pub struct UserAuthProfile {
    pub address: Address,                              // User's Stellar address
    pub preferred_method: AuthMethod,                  // Ed25519 or Secp256r1
    pub passkey_credentials: Vec<PasskeyCredential>,   // Up to 3 passkeys
    pub created_at: u64,                               // Profile creation
    pub last_active: u64,                             // Activity tracking
}
```

### Core Functions

#### Passkey Registration
```rust
fn register_passkey(
    env: Env,
    user: Address,
    public_key: BytesN<33>,
    credential_id: Bytes,
    origin: Symbol,
) -> Result<(), u32>
```

#### Passkey Authentication
```rust
fn authenticate_with_passkey(
    env: Env,
    user: Address,
    signature: PasskeySignature,
    credential_id: Bytes,
) -> Result<bool, u32>
```

#### Challenge Generation
```rust
fn generate_challenge(env: Env, user: Address) -> Bytes
```

## User Flow

### 1. Passkey Registration
1. User clicks "Enable Biometric Login"
2. Browser prompts for passkey creation
3. User authenticates with FaceID/TouchID
4. Passkey is generated and registered with smart contract
5. User's preferred auth method updates to Secp256r1

### 2. Biometric Transaction Signing
1. User initiates transaction (deposit, withdrawal, etc.)
2. Contract generates cryptographic challenge
3. Browser presents biometric authentication prompt
4. User authenticates with FaceID/TouchID
5. Passkey signs the challenge locally
6. Signed transaction is submitted to Stellar network
7. Contract verifies secp256r1 signature on-chain

## Security Features

### Multi-Factor Protection
- **Device Binding**: Passkeys are bound to specific devices and origins
- **Biometric Verification**: Local biometric authentication required
- **Challenge-Response**: Fresh cryptographic challenges for each transaction
- **Origin Validation**: Passkeys only work for registered domains

### Credential Management
- **Maximum 3 Passkeys**: Limits exposure if device is compromised
- **Credential Deactivation**: Lost/stolen devices can have credentials revoked
- **Fallback to Ed25519**: Users can always fall back to traditional signatures

### Anti-Phishing
- **Origin Binding**: Passkeys won't work on malicious websites
- **User Verification**: Biometric confirmation prevents unauthorized use
- **Challenge Freshness**: Replay attacks prevented by unique challenges

## Integration Guide

### Frontend Integration

#### Passkey Registration
```javascript
async function registerPasskey(userAddress, origin) {
    try {
        // Create passkey credential
        const credential = await navigator.credentials.create({
            publicKey: {
                challenge: new Uint8Array(32),
                rp: {
                    name: "SoroSusu",
                    id: origin
                },
                user: {
                    id: new TextEncoder().encode(userAddress),
                    name: userAddress,
                    displayName: userAddress
                },
                pubKeyCredParams: [
                    { alg: -7, type: "public-key" } // ES256 (secp256r1)
                ],
                authenticatorSelection: {
                    userVerification: "required",
                    residentKey: "preferred"
                }
            }
        });

        // Extract public key and credential ID
        const publicKey = credential.response.getPublicKey();
        const credentialId = credential.rawId;

        // Register with smart contract
        await contract.register_passkey(
            userAddress,
            publicKey,
            credentialId,
            origin
        );

        return { success: true };
    } catch (error) {
        console.error("Passkey registration failed:", error);
        return { success: false, error };
    }
}
```

#### Biometric Authentication
```javascript
async function authenticateWithPasskey(userAddress, challenge, credentialId) {
    try {
        // Get credential assertion
        const assertion = await navigator.credentials.get({
            publicKey: {
                challenge: challenge,
                allowCredentials: [{
                    id: credentialId,
                    type: "public-key",
                    transports: ["internal", "usb", "nfc", "ble"]
                }],
                userVerification: "required"
            }
        });

        // Extract signature data
        const signature = assertion.response.signature;
        const authData = assertion.response.authenticatorData;
        const clientData = assertion.response.clientDataJSON;

        // Authenticate with smart contract
        const isValid = await contract.authenticate_with_passkey(
            userAddress,
            { signature, auth_data: authData, client_data: clientData },
            credentialId
        );

        return { success: isValid };
    } catch (error) {
        console.error("Passkey authentication failed:", error);
        return { success: false, error };
    }
}
```

### Backend Integration

#### Enhanced Authentication Check
```rust
// Replace existing user.require_auth() calls with:
fn require_auth_enhanced(
    env: &Env,
    user: &Address,
    auth_method: Option<AuthMethod>,
    passkey_signature: Option<PasskeySignature>,
    credential_id: Option<Bytes>,
) -> Result<(), u32>
```

#### Transaction Flow
1. Check user's preferred authentication method
2. If Secp256r1, require passkey signature
3. If Ed25519, use traditional Stellar auth
4. Verify signature on-chain
5. Execute transaction if valid

## Migration Path

### For Existing Users
1. **No Action Required**: Existing Ed25519 users continue working unchanged
2. **Optional Enhancement**: Users can add passkeys as additional signers
3. **Gradual Adoption**: Passkeys become preferred method after registration

### For New Users
1. **Default Experience**: New users guided toward passkey setup
2. **Fallback Option**: Ed25519 still available for advanced users
3. **Hybrid Approach**: Users can maintain both authentication methods

## Testing

### Test Coverage
- ✅ Passkey registration flow
- ✅ Multiple credential management
- ✅ Authentication with valid signatures
- ✅ Challenge generation and verification
- ✅ Error handling for invalid credentials
- ✅ Preference management
- ✅ Event emission
- ✅ Security boundary testing

### Running Tests
```bash
# Run all passkey tests
cargo test --package sorosusu-contracts --lib passkey_auth_tests

# Run specific test
cargo test --package sorosusu-contracts --lib test_passkey_registration
```

## Deployment Considerations

### Protocol Requirements
- **Stellar Protocol 21+**: Required for secp256r1 verification
- **Soroban SDK v21.0.0+**: Includes passkey support
- **WebAuthn-compatible browsers**: Chrome, Safari, Firefox, Edge

### Security Audits
- **Smart Contract Audit**: Verify passkey signature verification logic
- **Frontend Security**: Review WebAuthn implementation
- **Key Management**: Ensure proper credential storage practices

### User Experience
- **Progressive Enhancement**: Start with Ed25519, add passkeys
- **Clear Onboarding**: Guide users through passkey setup
- **Fallback Support**: Maintain traditional auth options

## Benefits

### For Users
- **No Seed Phrases**: Eliminate risk of lost recovery phrases
- **Biometric Security**: Use familiar, secure authentication
- **Cross-Device Sync**: Passkeys available on all devices
- **Phishing Resistance**: Credentials bound to legitimate domains

### For SoroSusu
- **Reduced Friction**: Lower barrier to entry for mainstream users
- **Enhanced Security**: Multi-factor, hardware-backed authentication
- **Competitive Advantage**: Leading Web3 UX implementation
- **Regulatory Compliance**: Meets modern security standards

## Future Enhancements

### Multi-Signature Support
- Combine passkeys with traditional signatures
- Implement social recovery with passkey guardians
- Support hardware security keys (YubiKey, etc.)

### Advanced Features
- **Session Management**: Reduce frequent biometric prompts
- **Transaction Limits**: Automatically approve small transactions
- **Delegation**: Allow temporary access to trusted applications

### Cross-Platform
- **Mobile Apps**: Native iOS/Android passkey support
- **Desktop Clients**: Platform-specific integrations
- **Hardware Wallets**: Bridge to existing crypto hardware

## Troubleshooting

### Common Issues
1. **Unsupported Browser**: Ensure browser supports WebAuthn
2. **Device Compatibility**: Verify device has secure enclave
3. **Origin Mismatch**: Check domain configuration
4. **Expired Challenges**: Refresh challenge if verification fails

### Debug Information
- Check browser console for WebAuthn errors
- Verify Stellar protocol version (21+)
- Review contract events for registration/auth failures
- Validate credential format and encoding

## Conclusion

Passkey authentication represents a significant advancement in Web3 user experience, bringing the security and convenience of modern banking apps to decentralized finance. SoroSusu's implementation maintains full backward compatibility while providing a clear migration path to the future of blockchain authentication.

The combination of Stellar's native secp256r1 support and WebAuthn standards creates a robust, user-friendly authentication system that addresses one of the biggest barriers to mainstream cryptocurrency adoption.
