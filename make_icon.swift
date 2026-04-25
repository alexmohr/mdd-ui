#!/usr/bin/swift
import AppKit
import CoreImage

let iconSize  = 1024
let radius    = CGFloat(230)   // ~22% — matches macOS squircle
let padding   = CGFloat(160)   // whitespace around logo

let cwd     = FileManager.default.currentDirectoryPath
let inPath  = URL(fileURLWithPath: "icons/icon.png", relativeTo: URL(fileURLWithPath: cwd))
let outPath = URL(fileURLWithPath: "icons/icon.png", relativeTo: URL(fileURLWithPath: cwd))

guard let srcCI = CIImage(contentsOf: inPath) else {
    print("Error: could not load \(inPath.path)"); exit(1)
}

// Invert the logo (black strokes → white) then set alpha = luminance
// so white strokes are opaque and the white fill becomes transparent.
let inverted = srcCI.applyingFilter("CIColorInvert")
let logoCI   = inverted.applyingFilter("CIColorMatrix", parameters: [
    "inputRVector":    CIVector(x: 1,     y: 0,     z: 0,     w: 0),
    "inputGVector":    CIVector(x: 0,     y: 1,     z: 0,     w: 0),
    "inputBVector":    CIVector(x: 0,     y: 0,     z: 1,     w: 0),
    "inputAVector":    CIVector(x: 0.299, y: 0.587, z: 0.114, w: 0),
    "inputBiasVector": CIVector(x: 0,     y: 0,     z: 0,     w: 0),
])

let rep = NSBitmapImageRep(
    bitmapDataPlanes: nil, pixelsWide: iconSize, pixelsHigh: iconSize,
    bitsPerSample: 8, samplesPerPixel: 4, hasAlpha: true, isPlanar: false,
    colorSpaceName: .deviceRGB, bytesPerRow: 0, bitsPerPixel: 0)!

guard let nsCtx = NSGraphicsContext(bitmapImageRep: rep) else { print("No context"); exit(1) }
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current = nsCtx
let cg = nsCtx.cgContext

// Transparent base
cg.clear(CGRect(x: 0, y: 0, width: iconSize, height: iconSize))

// Dark rounded background (#171717 = neutral-900)
let bg = CGPath(roundedRect: CGRect(x: 0, y: 0, width: CGFloat(iconSize), height: CGFloat(iconSize)),
                cornerWidth: radius, cornerHeight: radius, transform: nil)
cg.addPath(bg); cg.setFillColor(CGColor(red: 0.09, green: 0.09, blue: 0.09, alpha: 1)); cg.fillPath()

// Clip all subsequent drawing to the rounded rect
cg.addPath(bg); cg.clip()

// Scale logo to fill padded area, keeping aspect ratio
let ext  = logoCI.extent
let area = CGRect(x: padding, y: padding,
                  width: CGFloat(iconSize) - 2*padding,
                  height: CGFloat(iconSize) - 2*padding)
let scale   = min(area.width / ext.width, area.height / ext.height)
let destW   = ext.width  * scale
let destH   = ext.height * scale
let destRect = CGRect(x: area.minX + (area.width  - destW) / 2,
                      y: area.minY + (area.height - destH) / 2,
                      width: destW, height: destH)

CIContext(cgContext: cg).draw(logoCI, in: destRect, from: ext)

NSGraphicsContext.restoreGraphicsState()

guard let png = rep.representation(using: .png, properties: [:]) else { print("PNG error"); exit(1) }
try! png.write(to: outPath)
print("✓ Written \(iconSize)×\(iconSize) rounded icon → \(outPath.path)")
