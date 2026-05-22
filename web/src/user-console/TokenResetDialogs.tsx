import { Button } from '../components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog'
import { Textarea } from '../components/ui/textarea'
import { selectAllReadonlyText } from '../lib/clipboard'
import type { TokenSecretCopyState } from '../components/TokenSecretField'
import type { RefObject } from 'react'
import type { EN } from './text'

type UserConsoleText = typeof EN

interface TokenResetDialogsProps {
  text: UserConsoleText
  resetTokenId: string | null
  resettingTokenId: string | null
  resetTokenError: string | null
  resetResultToken: string | null
  resetResultCopyState: TokenSecretCopyState
  resetResultFieldRef: RefObject<HTMLTextAreaElement>
  formatTemplate: (template: string, values: Record<string, string | number>) => string
  onCloseResetTokenDialog: () => void
  onResetToken: () => void
  onCloseResetResult: () => void
  onCopyResetResultToken: () => void
}

export default function TokenResetDialogs({
  text,
  resetTokenId,
  resettingTokenId,
  resetTokenError,
  resetResultToken,
  resetResultCopyState,
  resetResultFieldRef,
  formatTemplate,
  onCloseResetTokenDialog,
  onResetToken,
  onCloseResetResult,
  onCopyResetResultToken,
}: TokenResetDialogsProps): JSX.Element {
  return (
    <>
      <Dialog open={resetTokenId != null} onOpenChange={(open) => {
        if (!open) onCloseResetTokenDialog()
      }}>
        <DialogContent className="sm:max-w-[480px]">
          <DialogHeader>
            <DialogTitle>{text.tokens.resetDialog.title}</DialogTitle>
            <DialogDescription>
              {formatTemplate(text.tokens.resetDialog.description, {
                tokenId: resetTokenId ?? '',
              })}
            </DialogDescription>
          </DialogHeader>
          {resetTokenError ? (
            <p className="user-console-token-error" role="alert">{resetTokenError}</p>
          ) : null}
          <div className="table-actions justify-end">
            <Button type="button" variant="outline" onClick={onCloseResetTokenDialog} disabled={resettingTokenId != null}>
              {text.tokens.resetDialog.cancel}
            </Button>
            <Button type="button" variant="warning" onClick={onResetToken} disabled={resettingTokenId != null}>
              {resettingTokenId ? text.tokens.resetDialog.running : text.tokens.resetDialog.confirm}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
      <Dialog open={resetResultToken != null} onOpenChange={(open) => {
        if (!open) onCloseResetResult()
      }}>
        <DialogContent className="sm:max-w-[520px]">
          <DialogHeader>
            <DialogTitle>{text.tokens.resetResult.title}</DialogTitle>
            <DialogDescription>
              {resetResultCopyState === 'error'
                ? text.tokens.resetResult.copyBlocked
                : text.tokens.resetResult.copied}
            </DialogDescription>
          </DialogHeader>
          <label className="sr-only" htmlFor="user-console-reset-token-result">
            {text.tokens.resetResult.fieldLabel}
          </label>
          <Textarea
            id="user-console-reset-token-result"
            ref={resetResultFieldRef}
            readOnly
            rows={3}
            className="manual-copy-bubble-field min-h-[96px] resize-none font-mono text-xs"
            value={resetResultToken ?? ''}
            onClick={(event) => selectAllReadonlyText(event.currentTarget)}
            onFocus={(event) => selectAllReadonlyText(event.currentTarget)}
          />
          <div className="table-actions justify-end">
            <Button type="button" variant="outline" onClick={onCloseResetResult}>
              {text.tokens.resetResult.close}
            </Button>
            <Button type="button" onClick={onCopyResetResultToken}>
              {resetResultCopyState === 'copied'
                ? text.tokens.copied
                : resetResultCopyState === 'error'
                  ? text.tokens.resetResult.copyFailed
                  : text.tokens.resetResult.copy}
            </Button>
          </div>
        </DialogContent>
      </Dialog>
    </>
  )
}
