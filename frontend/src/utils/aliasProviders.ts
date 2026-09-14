const PROVIDER_LABELS: Record<string, string> = {
  simplelogin: 'SimpleLogin',
  duckduckgo: 'DuckDuckGo',
}

export function providerLabel(providerId: string): string {
  return PROVIDER_LABELS[providerId] ?? providerId
}
