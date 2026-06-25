import type { Meta, StoryObj } from '@storybook/react-vite'

import BrandLockup from './BrandLockup'

const meta = {
  title: 'Brand/Lockup',
  component: BrandLockup,
  parameters: {
    layout: 'centered',
  },
  args: {
    title: 'Tavily Hikari',
  },
} satisfies Meta<typeof BrandLockup>

export default meta

type Story = StoryObj<typeof meta>

export const Default: Story = {}

export const Compact: Story = {
  args: {
    compact: true,
  },
}
