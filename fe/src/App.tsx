import { useEffect, useState } from 'react'
import { Card, Text, Title, Grid } from '@mantine/core'

interface Assignment {
  name: string,
  status: string,
}

interface Report {
  id: number,
  assignments: Assignment[],
}

function App() {

  const [data, setData] = useState<Report[]>([]);

  useEffect(() => {
    fetch("http://localhost:8080/").then(
      (response) => {
        return response.json();
      }
    ).then((response: Report[]) => setData(response));
  }, []);
  console.log(data);

  return (
    <Grid gutter={10}>
      {
        data.map((report) =>
          <Grid.Col span={{ base: 12, sm: 4 }} key={report.id}>
            <Card shadow="sm" padding="lg" radius="md" withBorder>
              <Title order={4}>ID: {report.id}</Title>
              {report.assignments.map(
                (assignment, index) => (
                  <Text key={index}>
                    {assignment.name}: {assignment.status}
                  </Text>
                )
              )}
            </Card>
          </Grid.Col >
        )
      }
    </Grid >
  )
}

export default App
