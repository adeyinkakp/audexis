//
//  CloudTokenExchange.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation

extension CloudAuthService {
  func exchangeCodeForToken(code: String, verifier: String, redirectURI: String) async throws
    -> String
  {
    guard let tokenURL = URL(string: "\(authOrigin)/api/desktop/token"),
      let sessionURL = URL(string: "\(authOrigin)/api/desktop/session")
    else {
      throw CloudSignInError.invalidConfiguration
    }
    var request = URLRequest(url: tokenURL, timeoutInterval: 15)
    request.httpMethod = "POST"
    request.setValue("application/json", forHTTPHeaderField: "Content-Type")
    request.httpBody = try JSONEncoder().encode([
      "grant_type": "authorization_code", "code": code,
      "code_verifier": verifier, "redirect_uri": redirectURI,
    ])
    let (data, response) = try await networkSession.data(for: request)
    try Task.checkCancellation()
    guard let http = response as? HTTPURLResponse else { throw CloudSignInError.invalidResponse }
    guard http.statusCode == 200 else {
      if http.statusCode == 403 { throw CloudSignInError.subscriptionRequired }
      if http.statusCode >= 500 { throw CloudSignInError.serverUnavailable }
      throw CloudSignInError.expired
    }
    let token = try JSONDecoder().decode(CloudToken.self, from: data)
    guard token.token_type == "Bearer", Self.isValidToken(token.access_token) else {
      throw CloudSignInError.invalidResponse
    }
    var validation = URLRequest(url: sessionURL, timeoutInterval: 15)
    validation.setValue("Bearer \(token.access_token)", forHTTPHeaderField: "Authorization")
    let (_, validationResponse) = try await networkSession.data(for: validation)
    try Task.checkCancellation()
    guard let http = validationResponse as? HTTPURLResponse else {
      throw CloudSignInError.invalidResponse
    }
    guard http.statusCode == 200 else {
      if http.statusCode == 403 { throw CloudSignInError.subscriptionRequired }
      if http.statusCode >= 500 { throw CloudSignInError.serverUnavailable }
      throw CloudSignInError.invalidSession
    }
    return token.access_token
  }

  static func isValidToken(_ token: String) -> Bool {
    let prefix = "audexis_cloud_"
    guard token.hasPrefix(prefix) else { return false }
    let suffix = token.dropFirst(prefix.count)
    return suffix.count == 48 && suffix.allSatisfy { "0123456789abcdef".contains($0) }
  }
}

private struct CloudToken: Decodable {
  let access_token: String
  let token_type: String
}
