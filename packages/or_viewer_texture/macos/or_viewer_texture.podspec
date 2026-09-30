Pod::Spec.new do |s|
  s.name             = 'or_viewer_texture'
  s.version          = '0.1.0'
  s.summary          = 'Native CPU texture adapter for Opencut Reinforced.'
  s.description      = 'Presents leased CPU frames through Flutter external textures.'
  s.homepage         = 'https://github.com/huou07/opencut-reinforced'
  s.license          = { :type => 'MIT' }
  s.author           = { 'Opencut Reinforced' => 'maintainers@example.invalid' }
  s.source           = { :path => '.' }
  s.source_files     = 'or_viewer_texture/Sources/**/*'
  s.dependency 'FlutterMacOS'
  s.platform = :osx, '12.0'
  s.swift_version = '5.0'
  s.pod_target_xcconfig = { 'DEFINES_MODULE' => 'YES' }
end
