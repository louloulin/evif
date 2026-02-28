import React, { useRef, useEffect } from 'react'
import { Editor as MonacoEditor } from '@monaco-editor/react'
import type { editor } from 'monaco-editor'
import { Card } from '@/components/ui/card'
import { Button } from '@/components/ui/button'
import { Eye, EyeOff } from 'lucide-react'

interface MiniMapProps {
  content: string
  language: string
  theme?: 'vs-dark' | 'light'
  onPositionChange?: (position: { lineNumber: number; column: number }) => void
}

export const MiniMap: React.FC<MiniMapProps> = ({
  content,
  language,
  theme = 'vs-dark',
  onPositionChange,
}) => {
  const editorRef = useRef<editor.IStandaloneCodeEditor | null>(null)
  const [showMiniMap, setShowMiniMap] = React.useState(true)

  const handleEditorDidMount = (editor: editor.IStandaloneCodeEditor) => {
    editorRef.current = editor

    // 配置 MiniMap
    editor.updateOptions({
      minimap: {
        enabled: showMiniMap,
        side: 'right',
        showSlider: 'always',
        renderCharacters: true,
        maxColumn: 100,
      },
      scrollBeyondLastLine: false,
      readOnly: true,
      lineNumbers: 'off',
      glyphMargin: false,
      folding: false,
      lineDecorationsWidth: 0,
      lineNumbersMinChars: 0,
      overviewRulerLanes: 0,
      hideCursorInOverviewRuler: true,
      overviewRulerBorder: false,
    })

    // 监听光标位置变化
    if (onPositionChange) {
      editor.onDidChangeCursorPosition((e) => {
        onPositionChange(e.position)
      })
    }
  }

  useEffect(() => {
    if (editorRef.current) {
      editorRef.current.updateOptions({
        minimap: {
          enabled: showMiniMap,
        },
      })
    }
  }, [showMiniMap])

  return (
    <Card className="overflow-hidden">
      <div className="flex items-center justify-between px-3 py-2 border-b">
        <span className="text-sm font-medium">代码预览</span>
        <Button
          variant="ghost"
          size="sm"
          className="h-7 w-7 p-0"
          onClick={() => setShowMiniMap(!showMiniMap)}
          title={showMiniMap ? '隐藏小地图' : '显示小地图'}
        >
          {showMiniMap ? (
            <Eye className="h-4 w-4" />
          ) : (
            <EyeOff className="h-4 w-4" />
          )}
        </Button>
      </div>

      <div className="h-[200px]">
        <MonacoEditor
          height="200px"
          language={language}
          value={content}
          theme={theme}
          options={{
            readOnly: true,
            minimap: {
              enabled: showMiniMap,
            },
            scrollBeyondLastLine: false,
            lineNumbers: 'off',
            glyphMargin: false,
            folding: false,
            lineDecorationsWidth: 0,
            lineNumbersMinChars: 0,
          }}
          onMount={handleEditorDidMount}
        />
      </div>
    </Card>
  )
}
