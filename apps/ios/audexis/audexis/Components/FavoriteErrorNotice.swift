//
//  FavoriteErrorNotice.swift
//  audexis
//
//  Created by Kp on 10/29/26.
//

import SwiftUI

struct FavoriteErrorNotice: View {
  @Environment(CloudLibrary.self) private var library

  var body: some View {
    if let error = library.favoriteError {
      HStack(alignment: .top) {
        VStack(alignment: .leading, spacing: 4) {
          Text("Couldn’t update Favorites").font(.subheadline.weight(.semibold))
          Text(error).font(.caption)
        }
        Spacer(minLength: 8)
        Button {
          library.favoriteError = nil
        } label: {
          Image(systemName: "xmark").frame(width: 44, height: 44)
        }
        .accessibilityLabel("Dismiss Favorites error")
      }
      .foregroundStyle(AudexisStyle.destructive)
      .padding(12).background(AudexisStyle.danger_soft, in: RoundedRectangle(cornerRadius: 12))
      .accessibilityAddTraits(.updatesFrequently)
    }
  }
}
