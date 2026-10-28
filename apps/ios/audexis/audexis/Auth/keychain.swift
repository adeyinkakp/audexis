//
//  keychain.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation
import Security

final class KeychainHelper: Sendable {
  nonisolated static let standard = KeychainHelper()
  nonisolated private init() {}

  func save(_ data: String, service: String, account: String) throws {
    let query: [String: Any] = [
      kSecClass as String: kSecClassGenericPassword,
      kSecAttrService as String: service,
      kSecAttrAccount as String: account,
    ]
    let attributes: [String: Any] = [kSecValueData as String: Data(data.utf8)]
    let status = SecItemUpdate(query as CFDictionary, attributes as CFDictionary)
    if status == errSecItemNotFound {
      var item = query.merging(attributes) { _, new in new }
      item[kSecAttrAccessible as String] = kSecAttrAccessibleWhenUnlockedThisDeviceOnly
      guard SecItemAdd(item as CFDictionary, nil) == errSecSuccess else {
        throw ServerSettingsError.secureStorage
      }
    } else if status != errSecSuccess {
      throw ServerSettingsError.secureStorage
    }
  }

  nonisolated func read(service: String, account: String) -> String? {
    let query =
      [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
        kSecReturnData as String: true,
        kSecMatchLimit as String: kSecMatchLimitOne,
      ] as [String: Any]

    var dataTypeRef: AnyObject?
    let status: OSStatus = SecItemCopyMatching(query as CFDictionary, &dataTypeRef)

    if status == errSecSuccess, let data = dataTypeRef as? Data {
      return String(data: data, encoding: .utf8)
    }
    return nil
  }

  func delete(service: String, account: String) {
    let query =
      [
        kSecClass as String: kSecClassGenericPassword,
        kSecAttrService as String: service,
        kSecAttrAccount as String: account,
      ] as [String: Any]

    SecItemDelete(query as CFDictionary)
  }
}
