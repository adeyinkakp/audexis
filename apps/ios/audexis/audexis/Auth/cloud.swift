//
//  cloud.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import AuthenticationServices
import Foundation
import Observation
import UIKit

@Observable
@MainActor
class CloudAuthService {
  let authOrigin: String
  let networkSession: URLSession
  let callbackScheme = "audexis"
  var authenticationSession: ASWebAuthenticationSession?
  var presentationContext: PresentationAnchorProvider?
  var attemptID: UUID?
  var completionTask: Task<Void, Never>?
  var timeoutTask: Task<Void, Never>?
  var errorMessage: String?
  var isLoading = false

  init(authOrigin: String? = nil, networkSession: URLSession = .shared) {
    self.authOrigin = authOrigin ?? CloudAuthConfiguration.origin
    self.networkSession = networkSession
  }

  func signIn(authManager: AuthManager) {
    guard !isLoading else { return }
    errorMessage = nil
    let pkce = PKCE()
    let state = PKCE.generateState()
    let redirectURI = "\(callbackScheme)://callback"
    guard var components = URLComponents(string: "\(authOrigin)/auth/desktop") else {
      errorMessage = "Invalid authentication URL configuration."
      return
    }
    components.queryItems = [
      URLQueryItem(name: "redirect_uri", value: redirectURI),
      URLQueryItem(name: "state", value: state),
      URLQueryItem(name: "code_challenge", value: pkce.challenge),
      URLQueryItem(name: "code_challenge_method", value: "S256"),
    ]
    guard let authURL = components.url else {
      errorMessage = "Invalid authentication URL configuration."
      return
    }
    guard
      let window = UIApplication.shared.connectedScenes
        .compactMap({ $0 as? UIWindowScene })
        .filter({ $0.activationState == .foregroundActive })
        .compactMap({ $0.keyWindow }).first
    else {
      errorMessage = "Could not find the sign-in window. Please try again."
      return
    }
    let attempt = beginAttempt()
    presentationContext = PresentationAnchorProvider(window: window)
    let session = ASWebAuthenticationSession(url: authURL, callbackURLScheme: callbackScheme) {
      [weak self] callbackURL, error in
      Task { @MainActor [weak self] in
        self?.handleCallback(
          callbackURL: callbackURL, error: error, state: state,
          pkce: pkce, redirectURI: redirectURI,
          authManager: authManager, attempt: attempt, )
      }
    }
    session.presentationContextProvider = presentationContext
    authenticationSession = session
    if !session.start() {
      cancelSignIn(message: "Could not open the sign-in window. Please try again.")
    }
  }
}
