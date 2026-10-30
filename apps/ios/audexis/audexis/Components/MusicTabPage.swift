//
//  MusicTabPage.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct MusicTabPage<Content: View>: View {
  let title: String
  @Binding var showAccount: Bool
  @ViewBuilder var content: () -> Content

  var body: some View {
    NavigationStack {
      content()
        .safeAreaInset(edge: .top) { FavoriteErrorNotice().padding(.horizontal, 20) }
        .background(AudexisStyle.background)
        .navigationTitle(title)
        .toolbar {
          ToolbarItem(placement: .topBarTrailing) {
            Button {
              showAccount = true
            } label: {
              Image(systemName: "person.crop.circle").font(.title2)
            }.accessibilityLabel("Account")
          }
        }
    }
  }
}
