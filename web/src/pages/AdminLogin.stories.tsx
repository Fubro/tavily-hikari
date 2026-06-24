import type { Meta, StoryObj } from '@storybook/react-vite'

import { installDemoRuntime } from '../api/demo'
import { LanguageProvider } from '../i18n'
import { ThemeProvider } from '../theme'
import AdminLogin from './AdminLogin'

function AdminLoginStory(): JSX.Element {
  installDemoRuntime()
  return (
    <LanguageProvider>
      <ThemeProvider>
        <AdminLogin />
      </ThemeProvider>
    </LanguageProvider>
  )
}

const meta = {
  title: 'Public/Pages/AdminLogin',
  component: AdminLogin,
  parameters: {
    layout: 'fullscreen',
  },
  render: () => <AdminLoginStory />,
} satisfies Meta<typeof AdminLogin>

export default meta

type Story = StoryObj<typeof meta>

export const LightTheme: Story = {}

export const DarkTheme: Story = {
  globals: {
    themeMode: 'dark',
  },
}
