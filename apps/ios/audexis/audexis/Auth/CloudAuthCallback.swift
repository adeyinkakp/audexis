//
//  CloudAuthCallback.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import AuthenticationServices
import Foundation

extension CloudAuthService {
  func handleCallback(
    callbackURL: URL?, error: Error?, state: String, pkce: PKCE,
    redirectURI: String, authManager: AuthManager, attempt: UUID,
  ) {
    guard attemptID == attempt else { return }
    authenticationSession = nil
    presentationContext = nil
    armTimeout(for: attempt, seconds: 45)
    completionTask = Task {
      await completeSignIn(
        callbackURL: callbackURL, error: error, state: state,
        pkce: pkce, redirectURI: redirectURI, authManager: authManager, attempt: attempt, )
      finishAttempt(attempt)
    }
  }

  func completeSignIn(
    callbackURL: URL?, error: Error?, state: String, pkce: PKCE,
    redirectURI: String, authManager: AuthManager, attempt: UUID,
  ) async {
    guard attemptID == attempt, !Task.isCancelled else { return }
    if let error {
      if (error as? ASWebAuthenticationSessionError)?.code != .canceledLogin {
        errorMessage = "Could not open sign-in: \(error.localizedDescription)"
      }
      return
    }
    guard let callbackURL,
      let components = URLComponents(url: callbackURL, resolvingAgainstBaseURL: false),
      components.scheme == callbackScheme, components.host == "callback",
      components.path.isEmpty, components.user == nil, components.password == nil,
      components.port == nil, components.fragment == nil
    else {
      errorMessage = "Invalid sign-in callback."
      return
    }
    let items = components.queryItems ?? []
    guard items.filter({ $0.name == "state" }).count == 1,
      items.first(where: { $0.name == "state" })?.value == state
    else {
      errorMessage = "Could not verify this sign-in request. Please try again."
      return
    }
    if let error = items.first(where: { $0.name == "error" })?.value {
      errorMessage =
        error == "subscription_required"
        ? "An active Audexis Cloud subscription is required." : "Sign-in was cancelled or failed."
      return
    }
    guard items.filter({ $0.name == "code" }).count == 1,
      let code = items.first(where: { $0.name == "code" })?.value, !code.isEmpty
    else {
      errorMessage = "Authorization token not found."
      return
    }
    do {
      let token = try await exchangeCodeForToken(
        code: code, verifier: pkce.verifier, redirectURI: redirectURI, )
      try Task.checkCancellation()
      guard attemptID == attempt else { return }
      try authManager.login(token: token)
    } catch {
      guard attemptID == attempt, !Task.isCancelled else { return }
      if (error as? URLError)?.code == .timedOut {
        errorMessage = "The sign-in server took too long to respond. Please try again."
      } else {
        errorMessage = error.localizedDescription
      }
    }
  }
}
