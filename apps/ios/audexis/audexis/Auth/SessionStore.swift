//
//  SessionStore.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation

struct SavedSession: Codable {
  let connection: ServerConnection
  let credential: String
}

enum SessionStore {
  nonisolated private static let service = "com.kp.audexis.key"
  nonisolated private static let account = "user-session"

  static func read() -> SavedSession? {
    decode(storedValue())
  }

  static func restore() async -> SavedSession? {
    let value = await Task.detached(priority: .userInitiated) { storedValue() }.value
    return decode(value)
  }

  nonisolated private static func storedValue() -> String? {
    KeychainHelper.standard.read(service: service, account: account)
  }

  private static func decode(_ value: String?) -> SavedSession? {
    guard let value else { return nil }
    if let session = try? JSONDecoder().decode(SavedSession.self, from: Data(value.utf8)),
      !session.credential.isEmpty
    {
      return session
    }
    if value.hasPrefix("audexis_cloud_") {
      return SavedSession(
        connection: .init(kind: .cloud, url: CloudAuthConfiguration.origin), credential: value, )
    }
    return nil
  }

  static func save(_ session: SavedSession) throws {
    let data = try JSONEncoder().encode(session)
    try KeychainHelper.standard.save(
      String(decoding: data, as: UTF8.self), service: service, account: account, )
  }

  static func clear() {
    KeychainHelper.standard.delete(service: service, account: account)
  }
}
