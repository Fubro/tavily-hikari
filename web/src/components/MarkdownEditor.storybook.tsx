interface MarkdownEditorStorybookProps {
  id?: string
  name?: string
  value: string
  placeholder: string
  ariaLabelledBy?: string
  ariaDescribedBy?: string
  disabled?: boolean
  onChange: (value: string) => void
}

export default function MarkdownEditorStorybook({
  id,
  name,
  value,
  placeholder,
  ariaLabelledBy,
  ariaDescribedBy,
  disabled = false,
  onChange,
}: MarkdownEditorStorybookProps): JSX.Element {
  return (
    <div
      className="markdown-editor-shell markdown-editor-shell--storybook"
      aria-labelledby={ariaLabelledBy}
      aria-describedby={ariaDescribedBy}
    >
      <textarea
        id={id}
        name={name}
        className="textarea markdown-editor-storybook-input"
        value={value}
        aria-labelledby={ariaLabelledBy}
        aria-describedby={ariaDescribedBy}
        placeholder={placeholder}
        rows={7}
        maxLength={4000}
        disabled={disabled}
        onChange={(event) => onChange(event.target.value)}
      />
    </div>
  )
}
