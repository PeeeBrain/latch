import { AlertTriangle, ArrowRight, ExternalLink } from 'lucide-react'
import { HealthList } from './HealthList'

interface ReusedEntry {
  entry_id: string
  title: string
  username: string
}

interface ReusedPassword {
  entries: ReusedEntry[]
  count: number
}

interface ReusedPasswordsListProps {
  onSelectEntry: (entryId: string) => void
}

export default function ReusedPasswordsList({ onSelectEntry }: ReusedPasswordsListProps) {
  return (
    <HealthList<ReusedPassword>
      title="Reused Passwords"
      fetchKey="reused_passwords"
      emptyMessage="No reused passwords found!"
      instruction="These passwords are used on multiple accounts. Consider using unique passwords for each site."
      badgeColor="#ffa500"
      badgeBgColor="#ffa50020"
      renderBadge={(count) => `${count} set${count !== 1 ? 's' : ''}`}
      renderItem={(reused, _index) => (
        <button className="flex items-center justify-between gap-4 px-4 py-3 bg-theme-surface-hover border-2 border-theme-accent cursor-pointer transition-transform duration-100 shadow-theme hover:bg-theme-surface-hover hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm w-full text-left">
          <div className="flex items-center gap-3 text-theme-accent font-extrabold min-w-0 flex-1">
            <AlertTriangle size={18} />
            <span>Password used {reused.count} times</span>
          </div>
          <div className="flex items-center gap-2 text-theme-text-secondary text-sm">
            <span>{reused.entries.length} accounts</span>
            <ArrowRight size={16} />
          </div>
        </button>
      )}
      renderExpandedContent={(reused) => (
        <div>
          <div className="mb-4">
            <span className="block text-xs text-theme-text-secondary uppercase tracking-wider mb-2 font-semibold">Used on {reused.entries.length} accounts:</span>
            <div className="flex flex-col gap-2">
              {reused.entries.map((entry) => (
                <button
                  key={entry.entry_id}
                  onClick={(e) => {
                    e.stopPropagation()
                    onSelectEntry(entry.entry_id)
                  }}
                  className="flex items-center justify-between px-4 py-3 bg-theme-bg border-2 border-theme-accent text-theme-text font-extrabold font-theme cursor-pointer transition-transform duration-100 shadow-theme-sm hover:bg-theme-surface hover:translate-x-[2px] hover:translate-y-[2px] hover:shadow-theme-sm"
                >
                  <span>{entry.title}</span>
                  <ExternalLink size={14} className="text-theme-accent opacity-50 transition-opacity duration-200 group-hover:opacity-100" />
                </button>
              ))}
            </div>
          </div>
        </div>
      )}
    />
  )
}



