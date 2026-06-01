import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Skeleton } from "@/components/ui/skeleton";

interface PoolsListProps {
  isLoading?: boolean;
}

export function PoolsList({ isLoading = false }: PoolsListProps) {
  if (isLoading) {
    return (
      <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
        <CardHeader>
          <Skeleton className="h-5 w-32" />
        </CardHeader>
        <CardContent className="space-y-3">
          {Array.from({ length: 4 }).map((_, i) => (
            <div key={i} className="flex items-center justify-between p-3 rounded-lg bg-zinc-900/50">
              <div className="space-y-2">
                <Skeleton className="h-4 w-40" />
                <Skeleton className="h-3 w-24" />
              </div>
              <Skeleton className="h-6 w-16 rounded-full" />
            </div>
          ))}
        </CardContent>
      </Card>
    );
  }

  return (
    <Card className="bg-[#121212] border-none text-white h-full min-h-[400px]">
      <CardHeader>
        <CardTitle className="text-lg font-medium">Created Pools</CardTitle>
      </CardHeader>
      <CardContent>
        <div className="flex items-center justify-center h-[300px] text-zinc-600">
          {/* Empty state placeholder */}
          <p>No pools created yet!</p>
        </div>
      </CardContent>
    </Card>
  );
}
