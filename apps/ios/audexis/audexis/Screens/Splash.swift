//
//  Splash.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI

struct LoadingSplashScreen: View {
  var body: some View {
    Image("SplashIcon")
      .resizable()
      .scaledToFit()
      .frame(width: 128, height: 128)
      .accessibilityLabel("Audexis")
      .frame(maxWidth: .infinity, maxHeight: .infinity)
      .background(AudexisStyle.background)
      .ignoresSafeArea()
  }
}

#Preview { LoadingSplashScreen() }
