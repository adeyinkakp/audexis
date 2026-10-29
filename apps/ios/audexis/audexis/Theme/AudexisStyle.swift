//
//  AudexisStyle.swift
//  audexis
//
//  Created by Kp on 10/20/26.
//

import SwiftUI
import UIKit

enum AudexisStyle {
  static let background = adaptive(0xffffff, 0x0f0f0f)
  static let foreground = adaptive(0x2a2523, 0xf1f1f1)
  static let hover = adaptive(0xe0d6d3, 0x272727)
  static let hover_foreground = adaptive(0x2a2523, 0xf1f1f1)
  static let card = adaptive(0xfafafa, 0x181818)
  static let card_foreground = adaptive(0x2a2523, 0xf1f1f1)
  static let popover = adaptive(0xededed, 0x212121)
  static let active = adaptive(0xe0e0e0, 0x3f3f3f)
  static let popover_foreground = adaptive(0x2a2523, 0xf1f1f1)
  static let primary = adaptive(0xc44d4d, 0xe06464)
  static let primary_foreground = adaptive(0xffffff, 0xffffff)
  static let secondary = adaptive(0xe6e0de, 0x272727)
  static let secondary_foreground = adaptive(0x2a2523, 0xf1f1f1)
  static let muted = adaptive(0xebe6e4, 0x212121)
  static let muted_foreground = adaptive(0x756e6c, 0xaaaaaa)
  static let destructive = adaptive(0xd1453c, 0xf05248)
  static let destructive_foreground = adaptive(0xffffff, 0xffffff)
  static let border = adaptive(0xddd6d4, 0x303030)
  static let input = adaptive(0xddd6d4, 0x303030)
  static let accent = adaptive(0xeb6e6e, 0xf07878)
  static let accent_foreground = adaptive(0xffffff, 0x1b1110)
  static let ring = adaptive(0xeb6e6e, 0xf07878)
  static let scrollbar_track = adaptive(0xe6e0de, 0x0f0f0f)
  static let scrollbar_thumb = adaptive(0xc44d4d, 0x717171)
  static let primary_washed = adaptive(
    0xc44d4d, 0xe06464, lightOpacity: 26.0 / 255.0, darkOpacity: 31.0 / 255.0, )
  static let accent_washed = adaptive(
    0xeb6e6e, 0xf07878, lightOpacity: 26.0 / 255.0, darkOpacity: 31.0 / 255.0, )
  static let heading = adaptive(0x1a1614, 0xf1f1f1)
  static let warning = adaptive(0x9a6700, 0xe9b85f)
  static let surface_subtle = adaptive(0xebe6e4, 0x212121, lightOpacity: 0.25, darkOpacity: 0.25)
  static let surface_hover = adaptive(0xebe6e4, 0x212121, lightOpacity: 0.6, darkOpacity: 0.6)
  static let surface_glass = adaptive(0xffffff, 0x0f0f0f, lightOpacity: 0.3, darkOpacity: 0.3)
  static let surface_overlay = adaptive(0xffffff, 0x0f0f0f, lightOpacity: 0.75, darkOpacity: 0.75)
  static let surface_panel = adaptive(0xffffff, 0x0f0f0f, lightOpacity: 0.95, darkOpacity: 0.95)
  static let surface_popover = adaptive(0xededed, 0x212121, lightOpacity: 0.9, darkOpacity: 0.9)
  static let border_subtle = adaptive(0xddd6d4, 0x303030, lightOpacity: 0.5, darkOpacity: 0.5)
  static let content_secondary = adaptive(0x2a2523, 0xf1f1f1, lightOpacity: 0.7, darkOpacity: 0.7)
  static let content_faint = adaptive(0x2a2523, 0xf1f1f1, lightOpacity: 0.4, darkOpacity: 0.4)
  static let control_fill = adaptive(0x2a2523, 0xf1f1f1, lightOpacity: 0.15, darkOpacity: 0.15)
  static let primary_soft = adaptive(0xc44d4d, 0xe06464, lightOpacity: 0.2, darkOpacity: 0.2)
  static let primary_border = adaptive(0xc44d4d, 0xe06464, lightOpacity: 0.4, darkOpacity: 0.4)
  static let danger_soft = adaptive(0xd1453c, 0xf05248, lightOpacity: 0.1, darkOpacity: 0.1)
  static let danger_border = adaptive(0xd1453c, 0xf05248, lightOpacity: 0.4, darkOpacity: 0.4)
  static let warning_soft = adaptive(0x9a6700, 0xe9b85f, lightOpacity: 0.1, darkOpacity: 0.1)
  static let warning_border = adaptive(0x9a6700, 0xe9b85f, lightOpacity: 0.3, darkOpacity: 0.3)
  static let shade = adaptive(0x000000, 0x000000, lightOpacity: 0.2, darkOpacity: 0.2)
  static let highlight = adaptive(0xffffff, 0xffffff, lightOpacity: 0.1, darkOpacity: 0.1)
  static let focus_light = adaptive(0xffffff, 0xffffff, lightOpacity: 0.5, darkOpacity: 0.5)
  static let transparent = adaptive(0x000000, 0x000000, lightOpacity: 0.0, darkOpacity: 0.0)

  private static func adaptive(
    _ light: UInt32, _ dark: UInt32,
    lightOpacity: Double = 1, darkOpacity: Double = 1,
  ) -> Color {
    Color(
      uiColor: UIColor { traits in
        let isDark = traits.userInterfaceStyle == .dark
        let hex = isDark ? dark : light
        return UIColor(
          red: CGFloat((hex >> 16) & 255) / 255,
          green: CGFloat((hex >> 8) & 255) / 255,
          blue: CGFloat(hex & 255) / 255,
          alpha: CGFloat(isDark ? darkOpacity : lightOpacity), )
      })
  }
}
