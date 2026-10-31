//
//  CustomServerForm.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct CustomServerForm: View {
  @Environment(AuthManager.self) private var authManager
  @Environment(\.dismiss) private var dismiss
  @State private var serverURL = ""
  @State private var apiKey = ""
  @State private var errorMessage: String?
  @FocusState private var focusedField: Field?
  private enum Field { case url, key }

  var body: some View {
    NavigationStack {
      Form {
        Section {
          TextField("https://stream.audexis.app", text: $serverURL)
            .keyboardType(.URL).textContentType(.URL).textInputAutocapitalization(
              .never
            )
            .autocorrectionDisabled().focused($focusedField, equals: .url)
            .submitLabel(.next).onSubmit { focusedField = .key }
            .accessibilityLabel("Server URL")
        } header: {
          Text("Server URL")
        } footer: {
          Text(
            "Use your Audexis server’s full address, including its port if needed. Local network servers can use HTTP."
          )
        }
        Section {
          SecureField("Enter your API key", text: $apiKey)
            .textInputAutocapitalization(.never).autocorrectionDisabled()
            .focused($focusedField, equals: .key).submitLabel(.done)
            .onSubmit(save)
            .accessibilityLabel("Server API key")
        } header: {
          Text("API key")
        } footer: {
          Text("Your API key is saved securely in this device’s Keychain.")
        }
        if let errorMessage {
          Section { InlineError(message: errorMessage).listRowInsets(EdgeInsets()) }
        }
        Section {
          Button("Save server", action: save)
            .fontWeight(.semibold)
            .disabled(
              serverURL.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty
                || apiKey.trimmingCharacters(in: .whitespacesAndNewlines).isEmpty)
        } footer: {
          Text(
            "Your library will sync after connecting. You can update these details from your account."
          )
        }
      }
      .scrollContentBackground(.hidden).background(AudexisStyle.background)
      .navigationTitle("Custom server").navigationBarTitleDisplayMode(.inline)
      .toolbar {
        ToolbarItem(placement: .cancellationAction) { Button("Cancel") { dismiss() } }
      }
      .onAppear {
        serverURL =
          authManager.connection?.kind == .custom ? authManager.connection?.url ?? "" : ""
      }
    }.tint(AudexisStyle.primary)
  }

  private func save() {
    do {
      try authManager.saveCustomServer(url: serverURL, apiKey: apiKey)
      apiKey = ""
      dismiss()
    } catch { errorMessage = error.localizedDescription }
  }
}
