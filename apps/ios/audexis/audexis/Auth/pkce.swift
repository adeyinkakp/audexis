//
//  pkce.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import CryptoKit
import Foundation
import Security

struct PKCE {
  let verifier: String
  let challenge: String

  init() {
    var bytes = [UInt8](repeating: 0, count: 32)
    let _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)

    let verifierString = Data(bytes).base64UrlSafeNoPad()
    self.verifier = verifierString

    let hash = SHA256.hash(data: Data(verifierString.utf8))
    self.challenge = Data(hash).base64UrlSafeNoPad()
  }

  static func generateState() -> String {
    var bytes = [UInt8](repeating: 0, count: 32)
    let _ = SecRandomCopyBytes(kSecRandomDefault, bytes.count, &bytes)
    return Data(bytes).base64UrlSafeNoPad()
  }
}

extension Data {
  func base64UrlSafeNoPad() -> String {
    var base64 = self.base64EncodedString()
    base64 = base64.replacingOccurrences(of: "+", with: "-")
    base64 = base64.replacingOccurrences(of: "/", with: "_")
    base64 = base64.replacingOccurrences(of: "=", with: "")
    return base64
  }
}
