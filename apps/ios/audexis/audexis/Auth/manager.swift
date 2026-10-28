//
//  manager.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation
import Observation

enum AuthState {
  case checking, authenticated, unauthenticated
}

@Observable
@MainActor
class AuthManager {
  var state: AuthState = .checking
  private(set) var connection: ServerConnection?

  init() {}

  func checkExistingSession() async {
    let session = await SessionStore.restore()
    guard state == .checking, !Task.isCancelled else { return }
    connection = session?.connection
    state = connection == nil ? .unauthenticated : .authenticated
  }

  func login(token: String) throws {
    try save(
      .init(connection: .init(kind: .cloud, url: CloudAuthConfiguration.origin), credential: token))
  }

  func saveCustomServer(url: String, apiKey: String) throws {
    let serverURL = try ServerConnection.customURL(url)
    let key = apiKey.trimmingCharacters(in: .whitespacesAndNewlines)
    guard !key.isEmpty else { throw ServerSettingsError.missingKey }
    try save(
      .init(connection: .init(kind: .custom, url: serverURL.absoluteString), credential: key))
  }

  private func save(_ session: SavedSession) throws {
    try SessionStore.save(session)
    connection = session.connection
    state = .authenticated
  }

  func logout() {
    SessionStore.clear()
    connection = nil
    state = .unauthenticated
  }

  func remoteLogout() async {
    let session = SessionStore.read()
    logout()
    guard let session, session.connection.kind == .cloud,
      let revokeURL = URL(string: "\(session.connection.url)/api/desktop/revoke")
    else { return }
    var request = URLRequest(url: revokeURL)
    request.httpMethod = "POST"
    request.setValue("Bearer \(session.credential)", forHTTPHeaderField: "Authorization")
    _ = try? await URLSession.shared.data(for: request)
  }
}
