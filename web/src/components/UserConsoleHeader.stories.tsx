import type { Meta, StoryObj } from '@storybook/react-vite'

import UserConsoleHeader from './UserConsoleHeader'

const meta = {
  title: 'Console/UserConsoleHeader',
  component: UserConsoleHeader,
  parameters: {
    layout: 'padded',
  },
  args: {
    title: 'Tavily Hikari User Console',
    subtitle: 'Your account dashboard and token management',
    eyebrow: 'User Workspace',
    currentViewLabel: 'Current View',
    currentViewTitle: 'Overview',
    currentViewDescription: 'Track quotas, recent requests, and integration state.',
    sessionLabel: 'Signed in as',
    sessionDisplayName: 'Ivan',
    sessionProviderLabel: 'LinuxDo',
    adminLabel: 'Admin',
    isAdmin: true,
    adminHref: '/admin',
    adminActionLabel: 'Open Admin Dashboard',
    adminMenuLabel: 'Go to Admin',
    announcementsLabel: 'Open announcements',
    announcementCount: 2,
    logoutVisible: true,
    isLoggingOut: false,
    logoutLabel: 'Sign out',
    loggingOutLabel: 'Signing out…',
    onOpenAnnouncements: () => undefined,
    onLogout: () => undefined,
  },
} satisfies Meta<typeof UserConsoleHeader>

export default meta

type Story = StoryObj<typeof meta>

export const LightTheme: Story = {}

export const DarkTheme: Story = {
  globals: {
    themeMode: 'dark',
  },
}
