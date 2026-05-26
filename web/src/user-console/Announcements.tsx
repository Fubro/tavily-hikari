import type { Announcement } from '../api'
import { StatusBadge } from '../components/StatusBadge'
import { Button } from '../components/ui/button'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '../components/ui/dialog'
import {
  Drawer,
  DrawerContent,
  DrawerDescription,
  DrawerHeader,
  DrawerTitle,
} from '../components/ui/drawer'
import { Icon } from '../lib/icons'
import type { Language } from '../i18n'
import type { EN } from './text'

type UserConsoleText = typeof EN

interface UserConsoleAnnouncementsProps {
  language: Language
  text: UserConsoleText
  activeAnnouncements: Announcement[]
  historyAnnouncements: Announcement[]
  closedRecords: Record<string, number>
  historyOpen: boolean
  onHistoryOpenChange: (open: boolean) => void
  onCloseAnnouncement: (id: string) => void
}

interface UserConsoleAnnouncementsSectionProps extends UserConsoleAnnouncementsProps {
  hidden: boolean
}

function formatAnnouncementTime(value: number | null, language: Language): string {
  if (!value) return '-'
  try {
    return new Intl.DateTimeFormat(language === 'zh' ? 'zh-CN' : 'en-US', {
      month: 'short',
      day: 'numeric',
      hour: '2-digit',
      minute: '2-digit',
    }).format(new Date(value * 1000))
  } catch {
    return '-'
  }
}

function isClosed(item: Announcement, closedRecords: Record<string, number>): boolean {
  return closedRecords[item.id] != null
}

function announcementHistoryTime(item: Announcement): number | null {
  if (item.status === 'archived') {
    return item.archivedAt ?? item.publishedAt ?? item.updatedAt
  }
  return item.publishedAt ?? item.updatedAt
}

export default function UserConsoleAnnouncements({
  language,
  text,
  activeAnnouncements,
  historyAnnouncements,
  closedRecords,
  historyOpen,
  onHistoryOpenChange,
  onCloseAnnouncement,
}: UserConsoleAnnouncementsProps): JSX.Element {
  const strings = text.announcements
  const modalAnnouncement = activeAnnouncements.find((item) => item.displayKind === 'modal' && !isClosed(item, closedRecords))
    ?? null
  const tickerAnnouncement = activeAnnouncements.find((item) => item.displayKind === 'ticker' && !isClosed(item, closedRecords))
    ?? null

  return (
    <>
      {tickerAnnouncement ? (
        <section className="surface user-console-announcement-ticker" aria-live="polite">
          <div className="user-console-announcement-ticker-icon" aria-hidden="true">
            <Icon icon="mdi:bullhorn-outline" width={18} height={18} />
          </div>
          <div className="user-console-announcement-ticker-copy">
            <strong>{tickerAnnouncement.title}</strong>
            <span>{tickerAnnouncement.body}</span>
          </div>
          <Button
            type="button"
            variant="ghost"
            size="xs"
            className="user-console-announcement-close"
            aria-label={strings.tickerClose}
            onClick={() => onCloseAnnouncement(tickerAnnouncement.id)}
          >
            <Icon icon="mdi:close" width={16} height={16} aria-hidden="true" />
          </Button>
        </section>
      ) : null}

      <Dialog
        open={modalAnnouncement != null}
        onOpenChange={(open) => {
          if (!open && modalAnnouncement) {
            onCloseAnnouncement(modalAnnouncement.id)
          }
        }}
      >
        {modalAnnouncement ? (
          <DialogContent className="user-console-announcement-dialog">
            <DialogHeader>
              <DialogTitle>{modalAnnouncement.title}</DialogTitle>
              <DialogDescription>
                {strings.modalDescription}
              </DialogDescription>
            </DialogHeader>
            <div className="user-console-announcement-dialog-body">
              {modalAnnouncement.body}
            </div>
            <DialogFooter>
              <Button type="button" onClick={() => onCloseAnnouncement(modalAnnouncement.id)}>
                {strings.modalAcknowledge}
              </Button>
            </DialogFooter>
          </DialogContent>
        ) : null}
      </Dialog>

      <Drawer open={historyOpen} onOpenChange={onHistoryOpenChange} shouldScaleBackground={false}>
        <DrawerContent className="user-console-announcement-history">
          <DrawerHeader>
            <DrawerTitle>{strings.historyTitle}</DrawerTitle>
            <DrawerDescription>{strings.historyDescription}</DrawerDescription>
          </DrawerHeader>
          <div className="user-console-announcement-history-list">
            {historyAnnouncements.length === 0 ? (
              <div className="empty-state alert">{strings.emptyHistory}</div>
            ) : (
              historyAnnouncements.map((item) => (
                <article key={item.id} className="user-console-announcement-history-item">
                  <header>
                    <div>
                      <strong>{item.title}</strong>
                      <span>
                        {item.displayKind === 'ticker' ? strings.ticker : strings.modal}
                        {' · '}
                        {formatAnnouncementTime(announcementHistoryTime(item), language)}
                      </span>
                    </div>
                    <StatusBadge tone={item.status === 'published' ? 'success' : 'neutral'}>
                      {item.status === 'published' ? strings.published : strings.archived}
                    </StatusBadge>
                  </header>
                  <p>{item.body}</p>
                  {isClosed(item, closedRecords) ? (
                    <div className="user-console-announcement-closed">
                      <Icon icon="mdi:check-circle-outline" width={16} height={16} aria-hidden="true" />
                      <span>
                        {strings.closedAt.replace(
                          '{time}',
                          formatAnnouncementTime(closedRecords[item.id], language),
                        )}
                      </span>
                    </div>
                  ) : null}
                </article>
              ))
            )}
          </div>
        </DrawerContent>
      </Drawer>
    </>
  )
}

export function UserConsoleAnnouncementsSection({
  hidden,
  ...props
}: UserConsoleAnnouncementsSectionProps): JSX.Element | null {
  if (hidden) return null
  return <UserConsoleAnnouncements {...props} />
}
