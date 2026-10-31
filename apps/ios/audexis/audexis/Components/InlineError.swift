//
//  InlineError.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct InlineError: View {
  let message: String
  var body: some View {
    Label(message, systemImage: "exclamationmark.circle")
      .font(.subheadline).foregroundStyle(AudexisStyle.destructive)
      .frame(maxWidth: .infinity, alignment: .leading)
      .padding(14).background(AudexisStyle.danger_soft, in: RoundedRectangle(cornerRadius: 12))
      .accessibilityAddTraits(.updatesFrequently)
  }
}
