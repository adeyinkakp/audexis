//
//  CloudLibraryStatus.swift
//  audexis
//
//  Created by Kp on 10/26/26.
//

import SwiftUI

struct CloudLibraryStatus: View {
  @Environment(CloudLibrary.self) private var library

  var body: some View {
    if let error = library.error {
      VStack(alignment: .leading, spacing: 10) {
        Label(
          "Couldn’t refresh your library",
          systemImage: "exclamationmark.arrow.trianglehead.2.clockwise.rotate.90"
        )
        .font(.subheadline.weight(.semibold))
        Text(error).font(.caption).foregroundStyle(.secondary)
        Button("Try again") { Task { await library.refresh() } }
          .font(.subheadline.weight(.semibold))
      }
      .frame(maxWidth: .infinity, alignment: .leading).padding(16)
      .background(AudexisStyle.card, in: RoundedRectangle(cornerRadius: 16))
    } else if library.isLoading {
      ProgressView("Loading your library…").frame(maxWidth: .infinity).padding(24)
    } else if library.isRefreshing {
      HStack(spacing: 8) {
        ProgressView()
        Text("Updating library").font(.caption).foregroundStyle(.secondary)
      }
    }
  }
}
