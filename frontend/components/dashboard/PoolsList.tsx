import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";

export function PoolsList() {
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
