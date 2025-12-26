import { useEffect, useState, KeyboardEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
type CommandItem = {
  id: string;
  title: string;
  subtitle?: string;
};
type RustAppItem = {
  name: string;
  exec: string;
};

type SearchResult = {
  name: string;
  path:string;
  kind:string;
  score:number;
}


const ALL_ITEMS: CommandItem[] = [
  { id: "1", title: "Open Firefox", subtitle: "Browser" },
  { id: "2", title: "Open VS Code", subtitle: "Editor" },
  { id: "3", title: "Shutdown", subtitle: "System command" },
  { id: "4", title: "Reboot", subtitle: "System command" },
  { id: "5", title: "Open GitHub", subtitle: "Website" },
];

function App() {
  const [query, setQuery] = useState("");
  const [results, setResults] = useState<SearchResult[]>([]);
  const [allItems, setAllItems] = useState<SearchResult[]>([]);
  const [selectedIndex, setSelectedIndex] = useState(0);

  useEffect(()=>{
    const timeoutId = setTimeout(()=>{
      invoke("search_file",{query:query})
      .then((files)=>{
        console.log("Files found: ", files);
        setResults(files as SearchResult[]);
      })
      .catch((err)=>{
        console.log("Error searching files: ", err);
      })
    },300)

    return () => clearTimeout(timeoutId);
  },[query])
  useEffect(() => {
    const q = query.toLowerCase();
    const filtered = allItems.filter(
      (item) =>
        item.name.toLowerCase().includes(q) ||
        item.name?.toLowerCase().includes(q)
    );
    setResults(filtered);
    setSelectedIndex(0);
  }, [query,allItems]);

  const handleKeyDown = (e: KeyboardEvent<HTMLInputElement>) => {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      setSelectedIndex((prev) =>
        prev + 1 < results.length ? prev + 1 : prev
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      setSelectedIndex((prev) => (prev - 1 >= 0 ? prev - 1 : 0));
    } else if (e.key === "Enter") {
      e.preventDefault();
      const item = results[selectedIndex];
      console.log("Enter pressed on item: ", item);
      if (item) {
        invoke("start",{fileType:item.kind, path:item.path}).then(()=>{
          console.log('Launched App');
        }).catch((err)=>console.log(err,'was errror'))
      }
    } else if (e.key === "Escape") {
      console.log("Escape pressed");
    }
  };

  return (
    <div
      style={{
        height: "100vh",
        width: "100vw",
        display: "flex",
        alignItems: "center",
        justifyContent: "center",
        backgroundColor: "rgba(0,0,0,0.5)",
      }}
    >
      <div
        style={{
          width: "600px",
          borderRadius: "12px",
          backgroundColor: "#111827",
          padding: "12px 16px",
          boxShadow: "0 10px 30px rgba(0,0,0,0.4)",
          color: "white",
          fontFamily: "system-ui, -apple-system, BlinkMacSystemFont, sans-serif",
        }}
      >
        <input
          autoFocus
          placeholder="Type a command or app name..."
          value={query}
          onChange={(e) => setQuery(e.target.value)}
          onKeyDown={handleKeyDown}
          style={{
            width: "95%",
            padding: "8px 10px",
            borderRadius: "8px",
            border: "1px solid #374151",
            outline: "none",
            backgroundColor: "#020617",
            color: "white",
            marginBottom: "8px",
          }}
        />
        <div
          style={{
            maxHeight: "300px",
            overflowY: "auto",
          }}
        >
          {results.length === 0 && (
            <div style={{ padding: "8px", opacity: 0.7 }}>No results</div>
          )}
          {results.map((item, idx) => (
            <div
              key={item.score}
              style={{
                padding: "8px",
                borderRadius: "6px",
                marginBottom: "4px",
                backgroundColor:
                  idx === selectedIndex ? "#1f2937" : "transparent",
                cursor: "pointer",
              }}
            >
              <div style={{ fontSize: "14px" }}>{item.name}</div>
              {item.kind && (
                <div
                  style={{
                    fontSize: "12px",
                    opacity: 0.7,
                  }}
                >
                  {item.kind}
                </div>
              )}
            </div>
          ))}
        </div>
      </div>
    </div>
  );
}

export default App;

