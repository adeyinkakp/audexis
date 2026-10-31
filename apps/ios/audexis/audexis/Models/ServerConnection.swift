//
//  ServerConnection.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import Foundation

struct ServerConnection: Codable {
  enum Kind: String, Codable { case cloud, custom }
  let kind: Kind
  let url: String

  static func customURL(_ input: String) throws -> URL {
    guard let parts = URLComponents(string: input.trimmingCharacters(in: .whitespacesAndNewlines)),
      let host = parts.host, !host.isEmpty,
      parts.user == nil, parts.password == nil, parts.query == nil, parts.fragment == nil,
      parts.port.map({ (1...65535).contains($0) }) ?? true,
      let url = parts.url
    else { throw ServerSettingsError.invalidURL }
    let segments = host.split(separator: ".", omittingEmptySubsequences: false)
    let octets = segments.compactMap { Int($0) }
    let privateIPv4 =
      segments.count == 4 && octets.count == 4 && octets.allSatisfy { (0...255).contains($0) }
      && (octets[0] == 10 || octets[0] == 127
        || (octets[0] == 192 && octets[1] == 168)
        || (octets[0] == 169 && octets[1] == 254)
        || (octets[0] == 172 && (16...31).contains(octets[1])))
    let local = host == "localhost" || host == "[::1]" || host.hasSuffix(".local") || privateIPv4
    guard parts.scheme == "https" || (parts.scheme == "http" && local) else {
      throw ServerSettingsError.requiresHTTPS
    }
    return url
  }
}

enum ServerSettingsError: LocalizedError {
  case invalidURL, requiresHTTPS, missingKey, secureStorage
  var errorDescription: String? {
    switch self {
    case .invalidURL:
      "Enter a full server URL without credentials, query parameters, or a fragment."
    case .requiresHTTPS: "Use HTTPS, or HTP for a server on your local network."
    case .missingKey: "Enter your server API key."
    case .secureStorage: "Could not Securely save your credentials. Please try again."
    }
  }
}
