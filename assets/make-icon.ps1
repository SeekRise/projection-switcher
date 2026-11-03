# 生成应用图标源文件 icon.png（1024x1024，透明背景 + 圆角色块 + 显示器/双向箭头图形）
Add-Type -AssemblyName System.Drawing

$size = 1024
$bmp = New-Object System.Drawing.Bitmap($size, $size)
$g = [System.Drawing.Graphics]::FromImage($bmp)
$g.SmoothingMode = [System.Drawing.Drawing2D.SmoothingMode]::AntiAlias
$g.Clear([System.Drawing.Color]::Transparent)

# 圆角背景（Linear 紫 #5E6AD2）
$radius = 232
$d = $radius * 2
$path = New-Object System.Drawing.Drawing2D.GraphicsPath
$path.AddArc(0, 0, $d, $d, 180, 90)
$path.AddArc($size - $d, 0, $d, $d, 270, 90)
$path.AddArc($size - $d, $size - $d, $d, $d, 0, 90)
$path.AddArc(0, $size - $d, $d, $d, 90, 90)
$path.CloseFigure()
$accent = New-Object System.Drawing.SolidBrush([System.Drawing.Color]::FromArgb(94, 106, 210))
$g.FillPath($accent, $path)

# 显示器外框 + 底座
$white = [System.Drawing.Color]::White
$pen = New-Object System.Drawing.Pen($white, 34)
$pen.Alignment = [System.Drawing.Drawing2D.PenAlignment]::Inset
$g.DrawRectangle($pen, 252, 292, 520, 356)
$g.DrawLine($pen, 512, 648, 512, 706)
$g.DrawLine($pen, 396, 730, 628, 730)

# 屏幕内双向箭头（切换语义）
$arrow1 = New-Object System.Drawing.Pen($white, 28)
$arrow1.EndCap = [System.Drawing.Drawing2D.LineCap]::ArrowAnchor
$g.DrawLine($arrow1, 342, 424, 682, 424)
$arrow2 = New-Object System.Drawing.Pen($white, 28)
$arrow2.StartCap = [System.Drawing.Drawing2D.LineCap]::ArrowAnchor
$g.DrawLine($arrow2, 342, 528, 682, 528)

$g.Dispose()
$out = Join-Path $PSScriptRoot "icon.png"
$bmp.Save($out, [System.Drawing.Imaging.ImageFormat]::Png)
$bmp.Dispose()
Write-Host "saved: $out"
