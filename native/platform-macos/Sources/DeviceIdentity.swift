import Foundation
import Security
import CryptoKit

// A local identity, not a server enrollment or authorization grant. The private
// key never leaves the keychain helper; only public metadata is returned to UI.
struct Identity: Codable {
    let version: Int
    let deviceID: UUID
    let createdAt: Date
    let privateKey: Data

    init() {
        version = 1
        deviceID = UUID()
        createdAt = Date()
        privateKey = Curve25519.Signing.PrivateKey().rawRepresentation
    }

    func publicRecord() throws -> [String: Any] {
        guard version == 1 else { throw IdentityError.invalidRecord }
        let key = try Curve25519.Signing.PrivateKey(rawRepresentation: privateKey)
        let publicKey = key.publicKey.rawRepresentation
        // Stable, human-readable ID. Server enrollment must still reserve uniqueness.
        let digest = SHA256.hash(data: publicKey).prefix(8).map { String(format: "%02X", $0) }.joined()
        let groups = stride(from: 0, to: digest.count, by: 4).map { offset in
            String(digest.dropFirst(offset).prefix(4))
        }
        return ["schemaVersion": 1, "deviceId": deviceID.uuidString.lowercased(),
            "supportId": "DX-" + groups.joined(separator: "-"),
            "publicKey": publicKey.base64EncodedString(), "keyAlgorithm": "Ed25519",
            "createdAt": ISO8601DateFormatter().string(from: createdAt),
            "deviceName": Host.current().localizedName ?? "Bu Mac",
            "storage": "macOS Keychain", "registration": "local_only"]
    }
}
enum IdentityError: Error { case keychain(OSStatus), invalidRecord }

let query: [String: Any] = [
    kSecClass as String: kSecClassGenericPassword,
    kSecAttrService as String: "com.dengex.remote.lab.device",
    kSecAttrAccount as String: "identity-v1",
    kSecAttrSynchronizable as String: false
]
func existingIdentity() throws -> Identity? {
    var lookup = query
    lookup[kSecReturnData as String] = true
    lookup[kSecMatchLimit as String] = kSecMatchLimitOne
    var result: CFTypeRef?
    let status = SecItemCopyMatching(lookup as CFDictionary, &result)
    if status == errSecItemNotFound { return nil }
    guard status == errSecSuccess else { throw IdentityError.keychain(status) }
    guard let data = result as? Data else { throw IdentityError.invalidRecord }
    return try JSONDecoder().decode(Identity.self, from: data)
}
func loadOrCreateIdentity() throws -> Identity {
    if let existing = try existingIdentity() { return existing }
    let identity = Identity()
    var insert = query
    insert[kSecValueData as String] = try JSONEncoder().encode(identity)
    insert[kSecAttrLabel as String] = "dengeX Remote — yerel cihaz kimliği"
    insert[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
    let status = SecItemAdd(insert as CFDictionary, nil)
    if status == errSecDuplicateItem {
        // Simultaneous startup must return the identity which won the atomic insert.
        guard let winner = try existingIdentity() else { throw IdentityError.invalidRecord }
        return winner
    }
    guard status == errSecSuccess else { throw IdentityError.keychain(status) }
    return identity
}
do {
    let identity = try loadOrCreateIdentity()
    let record = try identity.publicRecord()
    FileHandle.standardOutput.write(try JSONSerialization.data(withJSONObject: record, options: [.sortedKeys]))
} catch {
    // Do not regenerate an unreadable identity, log its record, or fall back to plaintext.
    let message: String
    if case IdentityError.keychain(let status) = error {
        message = "keychain_unavailable:\(status)"
    } else { message = "device_identity_invalid" }
    FileHandle.standardError.write(Data(message.utf8))
    exit(1)
}
