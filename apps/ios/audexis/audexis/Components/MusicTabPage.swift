//
//  MusicTabPage.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct MusicTabPage<Content: View>: View {
  let title: String
  var path: Binding<[CloudCollectionDestination]>? = nil
  @State private var localPath: [CloudCollectionDestination] = []
  @Binding var showAccount: Bool
  @ViewBuilder var content: () -> Content

  var body: some View {
    NavigationStack(path: path ?? $localPath) {
      content()
        .navigationDestination(for: CloudCollectionDestination.self) { collection in
          CloudTrackCollectionPage(
            title: collection.title, subtitle: collection.subtitle, trackIDs: collection.trackIDs,)
        }
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
    .environment(
      \.openMusicCollection,
      { destination in
        if let path { path.wrappedValue.append(destination) } else { localPath.append(destination) }
      },)
  }
}
