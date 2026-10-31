//
//  PresentationAnchorProvider.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import AuthenticationServices
import UIKit

@MainActor
class PresentationAnchorProvider: NSObject, ASWebAuthenticationPresentationContextProviding {
  private let window: UIWindow

  init(window: UIWindow) {
    self.window = window
    super.init()
  }

  func presentationAnchor(for session: ASWebAuthenticationSession) -> ASPresentationAnchor {
    window
  }
}
