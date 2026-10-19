// SPDX-License-Identifier: GPL-3.0-only
// Adapted from samihsoylu/side-tray installer tooling.
import AppKit

// A small vector-drawn installation cue, rendered at Retina resolution.
let size=NSSize(width:540,height:420)
let bitmap=NSBitmapImageRep(bitmapDataPlanes:nil,pixelsWide:1080,pixelsHigh:840,
 bitsPerSample:8,samplesPerPixel:4,hasAlpha:true,isPlanar:false,colorSpaceName:.deviceRGB,
 bytesPerRow:0,bitsPerPixel:0)!
bitmap.size=size
NSGraphicsContext.saveGraphicsState()
NSGraphicsContext.current=NSGraphicsContext(bitmapImageRep:bitmap)
NSColor.white.setFill(); NSRect(origin:.zero,size:size).fill()
NSColor(calibratedWhite:0.55,alpha:1).setStroke()
let arrow=NSBezierPath(); arrow.lineWidth=3; arrow.lineCapStyle = .round; arrow.lineJoinStyle = .round
arrow.move(to:NSPoint(x:225,y:310)); arrow.line(to:NSPoint(x:315,y:310))
arrow.move(to:NSPoint(x:304,y:321)); arrow.line(to:NSPoint(x:315,y:310)); arrow.line(to:NSPoint(x:304,y:299)); arrow.stroke()
let caption="Drag Wireframe to Applications" as NSString
let attributes:[NSAttributedString.Key:Any]=[.font:NSFont.systemFont(ofSize:16,weight:.medium),.foregroundColor:NSColor(calibratedWhite:0.3,alpha:1)]
let width=caption.size(withAttributes:attributes).width
caption.draw(at:NSPoint(x:(540-width)/2,y:365),withAttributes:attributes)
NSGraphicsContext.restoreGraphicsState()
try bitmap.representation(using:.tiff,properties:[:])!.write(to:URL(fileURLWithPath:CommandLine.arguments[1]))
