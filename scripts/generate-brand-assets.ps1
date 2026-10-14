# Original vector geometry; keep in sync with assets/branding/mark.svg.
$ErrorActionPreference = 'Stop'
$projectDir = Split-Path -Parent $PSScriptRoot
$brandDir = Join-Path $projectDir 'assets\branding'
New-Item -ItemType Directory -Path $brandDir -Force | Out-Null
Add-Type -AssemblyName System.Drawing
function New-BrandBitmap([int]$Size) {
    $large = [Drawing.Bitmap]::new(1024,1024)
    $g = [Drawing.Graphics]::FromImage($large)
    $g.SmoothingMode = [Drawing.Drawing2D.SmoothingMode]::AntiAlias
    $g.ScaleTransform(4,4)
    $g.Clear([Drawing.Color]::Transparent)
    $path = [Drawing.Drawing2D.GraphicsPath]::new()
    foreach ($arc in @(@(0,0,180),@(148,0,270),@(148,148,0),@(0,148,90))) { $path.AddArc($arc[0],$arc[1],108,108,$arc[2],90) }
    $path.CloseFigure()
    $brush = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml('#142e32'))
    $g.FillPath($brush,$path); $brush.Dispose(); $path.Dispose()
    foreach ($face in @(
        @{Color='#246e6b'; Points=@(45,99,128,147,128,207,45,159)},
        @{Color='#35a18f'; Points=@(128,147,211,99,211,159,128,207)},
        @{Color='#173c3e'; Points=@(45,99,128,51,211,99,128,147)}
    )) {
        $points = [Drawing.PointF[]]@(for($i=0;$i -lt $face.Points.Count;$i+=2){[Drawing.PointF]::new($face.Points[$i],$face.Points[$i+1])})
        $brush = [Drawing.SolidBrush]::new([Drawing.ColorTranslator]::FromHtml($face.Color))
        $g.FillPolygon($brush,$points); $brush.Dispose()
    }
    $pen = [Drawing.Pen]::new([Drawing.ColorTranslator]::FromHtml('#83e6cc'),9)
    $pen.LineJoin = [Drawing.Drawing2D.LineJoin]::Round
    $pen.StartCap = $pen.EndCap = [Drawing.Drawing2D.LineCap]::Round
    $g.DrawPolygon($pen,[Drawing.PointF[]]@([Drawing.PointF]::new(45,99),[Drawing.PointF]::new(128,51),[Drawing.PointF]::new(211,99),[Drawing.PointF]::new(128,147)))
    $g.DrawLine($pen,86.5,75,169.5,123); $g.DrawLine($pen,169.5,75,86.5,123); $pen.Dispose()
    $pen = [Drawing.Pen]::new([Drawing.ColorTranslator]::FromHtml('#51bba6'),5)
    $pen.LineJoin = [Drawing.Drawing2D.LineJoin]::Round
    $g.DrawLines($pen,[Drawing.PointF[]]@([Drawing.PointF]::new(45,99),[Drawing.PointF]::new(45,159),[Drawing.PointF]::new(128,207),[Drawing.PointF]::new(211,159),[Drawing.PointF]::new(211,99)))
    $g.DrawLine($pen,128,147,128,207); $pen.Dispose(); $g.Dispose()
    $bitmap = [Drawing.Bitmap]::new($Size,$Size)
    $g = [Drawing.Graphics]::FromImage($bitmap)
    $g.InterpolationMode = [Drawing.Drawing2D.InterpolationMode]::HighQualityBicubic
    $g.DrawImage($large,0,0,$Size,$Size); $g.Dispose(); $large.Dispose()
    return $bitmap
}
$preview = New-BrandBitmap 512
$preview.Save((Join-Path $brandDir 'mark.png'),[Drawing.Imaging.ImageFormat]::Png)
$preview.Dispose()
$bitmap = New-BrandBitmap 128
$rgba = [byte[]]::new(128*128*4)
for($y=0;$y -lt 128;$y++){for($x=0;$x -lt 128;$x++){
    $c=$bitmap.GetPixel($x,$y); $i=($y*128+$x)*4
    $rgba[$i]=$c.R; $rgba[$i+1]=$c.G; $rgba[$i+2]=$c.B; $rgba[$i+3]=$c.A
}}
[IO.File]::WriteAllBytes((Join-Path $brandDir 'mark.rgba'),$rgba); $bitmap.Dispose()
$sizes=@(16,24,32,48,64,128,256)
$images=@(foreach($size in $sizes){
    $bitmap=New-BrandBitmap $size; $stream=[IO.MemoryStream]::new()
    $bitmap.Save($stream,[Drawing.Imaging.ImageFormat]::Png)
    ,$stream.ToArray(); $stream.Dispose(); $bitmap.Dispose()
})
$stream=[IO.File]::Create((Join-Path $projectDir 'assets\GridfinityStudio.ico'))
$writer=[IO.BinaryWriter]::new($stream)
try {
    $writer.Write([uint16]0); $writer.Write([uint16]1); $writer.Write([uint16]$sizes.Count)
    $offset=6+16*$sizes.Count
    for($i=0;$i -lt $sizes.Count;$i++){
        $dimension=if($sizes[$i] -eq 256){0}else{$sizes[$i]}
        $writer.Write([byte]$dimension); $writer.Write([byte]$dimension)
        $writer.Write([byte]0); $writer.Write([byte]0); $writer.Write([uint16]1); $writer.Write([uint16]32)
        $writer.Write([uint32]$images[$i].Length); $writer.Write([uint32]$offset)
        $offset+=$images[$i].Length
    }
    foreach($bytes in $images){$writer.Write([byte[]]$bytes)}
} finally {$writer.Dispose(); $stream.Dispose()}
Write-Output 'Logo, RGBA texture and multi-resolution Windows icon generated.'
