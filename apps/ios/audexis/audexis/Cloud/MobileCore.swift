//
//  MobileCore.swift
//  audexis
//
//  Created by Kp on 10/21/26.
//

import Foundation

private nonisolated struct MobileResponse<Body: Decodable>: Decodable {
  let ok: Body?
  let error: String?
}

actor MobileCore {
  private final class Handle: @unchecked Sendable {
    let pointer = audexis_mobile_create()!
    deinit { audexis_mobile_destroy(pointer) }
  }
  private let handle = Handle()

  func openLibrary(at directory: URL) throws -> CloudSnapshot {
    try call(.init(op: "open", directory: directory.path))
  }

  func librarySnapshot() throws -> CloudSnapshot {
    try call(.init(op: "snapshot"))
  }

  func call<T: Decodable & Sendable>(_ command: MobileCommand, as type: T.Type = T.self) throws
    -> T
  {
    let bytes = try JSONEncoder().encode(command)
    let result = String(decoding: bytes, as: UTF8.self).withCString {
      audexis_mobile_call(handle.pointer, $0)
    }

    guard let result else { throw CloudError.message("Could not open the music library.") }
    defer { audexis_mobile_string_free(result) }
    let data = Data(bytes: result, count: strlen(result))
    let response = try JSONDecoder().decode(MobileResponse<T>.self, from: data)
    if let error = response.error { throw CloudError.message(error) }
    guard let value = response.ok else { throw CloudError.message("Invalid library response.") }
    return value
  }
}
