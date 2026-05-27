import { useEffect, useRef, useState } from 'react'
import { Crepe, CrepeFeature } from '@milkdown/crepe'
import '@milkdown/crepe/theme/frame.css'

import { Textarea } from './ui/textarea'

interface MarkdownEditorProps {
  id?: string
  name?: string
  value: string
  placeholder: string
  ariaLabelledBy?: string
  ariaDescribedBy?: string
  disabled?: boolean
  onChange: (value: string) => void
}

export default function MarkdownEditor({
  id,
  name,
  value,
  placeholder,
  ariaLabelledBy,
  ariaDescribedBy,
  disabled = false,
  onChange,
}: MarkdownEditorProps): JSX.Element {
  const rootRef = useRef<HTMLDivElement | null>(null)
  const editorRef = useRef<Crepe | null>(null)
  const onChangeRef = useRef(onChange)
  const [fallback, setFallback] = useState(false)

  useEffect(() => {
    onChangeRef.current = onChange
  }, [onChange])

  useEffect(() => {
    const root = rootRef.current
    if (!root) return undefined

    let cancelled = false
    root.innerHTML = ''
    setFallback(false)

    const editor = new Crepe({
      root,
      defaultValue: value,
      features: {
        [CrepeFeature.Cursor]: true,
        [CrepeFeature.BlockEdit]: false,
        [CrepeFeature.Toolbar]: false,
        [CrepeFeature.Placeholder]: true,
        [CrepeFeature.ListItem]: true,
        [CrepeFeature.LinkTooltip]: false,
        [CrepeFeature.Table]: true,
        [CrepeFeature.CodeMirror]: true,
        [CrepeFeature.ImageBlock]: false,
        [CrepeFeature.Latex]: false,
      },
      featureConfigs: {
        [CrepeFeature.Placeholder]: {
          text: placeholder,
          mode: 'block',
        },
      },
    })

    editor.on((api) => {
      api.markdownUpdated((_ctx, markdown, prevMarkdown) => {
        if (markdown !== prevMarkdown) onChangeRef.current(markdown)
      })
    })

    editor.create()
      .then(() => {
        if (cancelled) {
          void editor.destroy()
          return
        }
        editorRef.current = editor
        editor.setReadonly(disabled)
      })
      .catch(() => {
        if (!cancelled) setFallback(true)
      })

    return () => {
      cancelled = true
      editorRef.current = null
      void editor.destroy()
    }
  }, [])

  useEffect(() => {
    editorRef.current?.setReadonly(disabled)
  }, [disabled])

  if (fallback) {
    return (
      <Textarea
        id={id}
        name={name}
        value={value}
        aria-labelledby={ariaLabelledBy}
        aria-describedby={ariaDescribedBy}
        placeholder={placeholder}
        rows={7}
        maxLength={4000}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      />
    )
  }

  return (
    <div
      id={id}
      className="markdown-editor-shell"
      aria-labelledby={ariaLabelledBy}
      aria-describedby={ariaDescribedBy}
    >
      <div ref={rootRef} />
    </div>
  )
}
